# 工单 02 报告：搜索 —— 标题/正文，中文可用

执行日期：2026-09-26。状态：**完成**。门槛、本地核心验证、WSLg 与 Windows 原生冒烟全部通过；改动随本工单提交（见 git log）。范围：列表面板搜索框即时过滤标题+正文；trigram（≥3 字符）与 `LIKE` 兜底（<3 字符）双路径；FTS 与条目同事务维护；软删除过滤；诚实空状态。

## 一、交付物

| 类别 | 位置 |
|---|---|
| 查询规划 | `crates/promptdeck-core/src/search.rs`：`plan()` → `QueryPlan::{All, Trigram, Like}`；FTS 短语引号转义；`LIKE` 通配符/反斜杠转义（`ESCAPE '\'`）；查询按字面处理，无高级语法 |
| 仓储搜索 | `crates/promptdeck-core/src/storage/library.rs`：`search_prompts()`（trigram JOIN + 软删除过滤；`LIKE` 兜底；排序与列表一致）；`sync_fts`/`remove_fts` 随 `create_prompt`/`save`/`soft_delete` 同事务；打开时索引行数与在用条目数不一致则按 rowid 重建；三处查询共用 `SUMMARY_COLUMNS`/`SUMMARY_ORDER` |
| app | `controller.rs` 新增 `search_edited()`，`refresh_items()` 读取 `search-query` 即时 `search_prompts` 并维护 `search-empty-title`/`library-empty`；`strings.rs` 新增搜索占位与空态文案及 `empty_search_title(q)` |
| UI | `ui/library.slint`：`SearchField`（`Theme.control` 底、聚焦 accent 下划线、占位随 preedit 隐藏）、`EmptyState` 复用；`ui/app.slint` 属性/回调接线；`ui/strings.slint` 新文案 |

## 二、实现要点

- **双路径**：`chars().count() < 3` 走 `LIKE '%q%'`（转义 `\`/`%`/`_`），否则 FTS5 `MATCH` 一个双引号短语（内部引号翻倍）；两条路径语义一致——**连续子串**匹配。中文 2 字（如「翻译」）因此可命中。
- **索引一致性**：`items_fts` 用 `items` 的隐式 rowid 关联（`item_id` 列仅作 JOIN 键）；保存即「删旧行 + 插新行」，软删除即删行；搜索两条路径都额外过滤 `deleted_at IS NULL`（防御）。
- **旧库兼容**：FTS 维护上线前写入的库没有任何索引行；打开时行数与在用条目数不等即整体重建（同样修复部分写入的索引）。
- **UI 纪律**（hallmark，`design.md` 锁定系统内）：搜索框只使用 tokens，无硬编码颜色；聚焦以 accent 下划线提示（沿用标题输入先例）；无结果空态含查询词，库为空仍给「Ctrl+N 新建」引导；不新造图标与装饰。

## 三、验收对照（工单 8 条）

| # | 验收项 | 状态 |
|---|---|---|
| 1 | 输入即过滤，清空恢复全量 | 通过（双端冒烟） |
| 2 | 中文 ≥3 字子串命中（trigram） | core 测试通过（`search_finds_cjk_substrings_from_trigrams` 等） |
| 3 | 英文大小写不敏感（trigram） | core 测试通过（`search_is_case_insensitive_for_english_queries`） |
| 4 | <3 字符回退 `LIKE` 且命中 | core 测试通过（短查询含 `%`、`_`、`"hi"` 字面处理） |
| 5 | FTS 保存/软删除同事务同步；软删除不出现在结果 | core 测试通过（双路径删除过滤 + 保存后旧词失配/新词命中） |
| 6 | 无结果诚实空状态（含查询词） | 通过：`没有找到「q」` + 「换个关键词，或清空搜索查看全部」（双端截图） |
| 7 | core 单测 + fmt/clippy/test 全绿 | 通过：fmt OK、clippy 0 警告、**42 passed** |
| 8 | WSLg 与 Windows 冒烟（2 字短词命中） | 通过（WSLg：报整/周报整理/空态/清空；Windows：周报/小写 promptdeck/trigram/空态/清空） |

## 四、验证证据

- **门槛**：`cargo fmt --check` 通过；`cargo clippy --workspace --all-targets` 0 警告；`cargo test --workspace` 42 passed（较 01 新增 15 个搜索相关用例）。
- **本地核心补验**（不涉及用户桌面）：以 Windows 开发库副本（`/tmp/opencode/pd-win-db2/`，FTS 维护前写入）经临时集成测试打开——索引自动重建后，2 字「周报」`LIKE` 命中「周报整理」、4 字「周报整理」trigram 命中、正文「自动保存」命中；测试跑完已删除。
- **双端冒烟**：
  - WSLg（Linux debug）：搜索框渲染占位「搜索标题与正文…」；2 字「报整」→ 仅「周报整理」；「周报整理」→ trigram 命中；「不存在的词」→「没有找到「不存在的词」」空态；清空 → 两条恢复（`t02-wsl-01`–`05`）。
  - Windows 原生（release，MSVC 构建）：2 字「周报」命中；小写「promptdeck」命中正文「PromptDeck」（大小写不敏感）；「周报整理」trigram 命中；「找不到的词」空态；清空恢复两条（`t02-win-01`–`06`）。暗色主题下 token 与聚焦下划线正常。
  - 真实库索引回填：Windows 库 `items_fts` 行数 = 在用条目数 = 2（含空条目），搜索未改动任何用户内容。
- **code-review 双轴**：Standards 与 Spec 两路审查，采纳的修复：软删除测试补 trigram 路径；空查询测试改为显式顺序断言（原为同义反复）；标题/正文 FTS 各用独立 3+ 字词验证；回填测试改用 trigram 查询并新增「部分索引重建」用例；移除 `RefCell` 镜像状态改读 UI 属性；空态判定改用 `library-empty` 布尔而非显示字符串；`search` 模块收敛公有面；SQL 列/排序常量化。未采纳（记录理由）：搜索框 hover 态——Slint `TextInput` 无 hover，覆盖层会破坏点击定位光标；已在 `docs/design-foundations.md` 明确输入组件豁免（与 Slint `LineEdit`、既有标题输入一致）。
- **文档同步**：`docs/data-model.md`（索引不一致重建）、`docs/design-foundations.md`（输入组件状态与焦点指示口径）。

## 五、未决事项

1. 搜索随输入即时查询（每条按键一次本地 SQLite 查询）；库量级为此设计的边界，若未来到万级条目再评估防抖。
2. 当前搜索不持久化、不参与选中；筛选状态下 Ctrl+N 新建的条目可能不匹配当前查询而不出现在列表（画布仍选中）——留待工单 07 键盘/状态收尾统一处理。
3. 工单勾选沿用 01 的约定保持原样，完成证据在本报告与工单 `## Comments` 中。

## 六、下一步建议

按顺序进入 03（Markdown 只读渲染与模式切换），完成后同样以 fmt/clippy/test + 双端冒烟闭环。
