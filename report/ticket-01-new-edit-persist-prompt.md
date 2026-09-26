# 工单 01 完成报告：新建、编辑并持久化第一条 Prompt（含正确外壳与主题 tokens）

执行日期：2026-09-26。状态：**完成**（工作区未 commit）。范围：Slice 01 的 tracer bullet —— 一次做对外壳（N3 side-rail、224px 列表面板、单画布、左下模式切换器、design.md tokens），行为只做最小闭环（新建 → 编辑 → 自动保存 → 重启恢复）。不含 Blocks/Composition、Radar/Inbox、Versions UI、发布体系。

## 一、交付物

| 类别 | 位置 |
|---|---|
| 领域模型 | `crates/promptdeck-core/src/{model,error,clock,id}.rs`（Item/ItemSummary/Revision、`derive_title`、FNV-1a 64 `content_hash`、Clock/IdSource 注入） |
| 存储 | `crates/promptdeck-core/src/storage/mod.rs`（schema v1 一次性迁移：items/tags/item_tags/variables/versions/settings/items_fts trigram）、`storage/library.rs`（Library 仓储 + 设置读写 + 单测） |
| 装配/控制 | `crates/promptdeck-app/src/{main,controller,strings,theme}.rs`（800ms 防抖自动保存、选中恢复、主题循环与持久化、中文文案常量） |
| UI | `crates/promptdeck-app/ui/{app,shell,library,canvas,theme,strings,icons}.slint` |
| 图标 | `ui/icons/*.svg`（iconfont cid=19238：library/inbox/settings/add/theme×3/rail-toggle×2）、`ui/icons.slint` 的 `AppIcon`（`Image.colorize` 按 tokens 着色） |
| 应用图标 | `ui/assets/app-icon.png`（用户设计稿，1254→512px，`Window.icon`） |
| 依赖新增 | workspace：`thiserror 2`、`uuid v7`、`chrono 0.4`（clock/std）；core：`rusqlite 0.40`（bundled） |
| 文档同步 | `docs/design-foundations.md`（图标来源、行距、焦点指示决策、rail 切换方式）、工单 `.scratch/slice-01-library-loop/issues/01-*.md` 追加 `## Comments` |

存储约定：SQLite 唯一真源（ADR-0002）；`PRAGMA user_version=1`；设置键 `theme`、`library.selected_prompt`、`rail.expanded`；开发期数据库可删除重建。

## 二、实现要点

- **首次保存**：新建即写 1 条 version；之后按 `content_hash(title+body)` 去重，且距上一 version ≥10s 才追加（注入时钟单测覆盖）。
- **标题推导**：标题为空时取正文首个标题行/非空行（去 `#`、截断 80 字符）；画布头部可随时手改。
- **自动保存**：编辑 800ms 防抖；切换选中、退出前 flush；重启后正文 Markdown 逐字一致。
- **主题**：`design.md` 的 OKLCH 一次性转 sRGB 落 `ui/theme.slint`（Light/Azure、Dark/Forest），默认跟随系统、可循环切换并持久化；主题之外无硬编码颜色/字体。
- **交互打磨（经确认的调整）**：移除焦点环与聚焦高亮（同步修订 design-foundations 与工单第 8 条）；选中指示条即时切换（去掉 160ms 宽度动画）；列表行内容 54px、行距 62px，1px `rule` 分隔线上下各留 4px；侧栏改为底部按钮切换展开/收起（`«`/`»`）并持久化，不再随 hover 自动展开。
- **图标**：Unicode 字形全部替换为单色 SVG，经 `AppIcon` 自动随主题着色；「跟随系统」图标为用户提供的太阳+月牙（`fill-rule="evenodd"`）。

## 三、验收对照（工单 12 条）

| # | 验收项 | 状态 |
|---|---|---|
| 1 | schema v1 迁移、二次启动幂等 | 完成（迁移单测） |
| 2 | Ctrl+N 新建并聚焦画布；标题推导/可改 | 完成（`acc-02/03`、`win-acc-02/11`） |
| 3 | 800ms 防抖保存；重启逐字一致 | 完成（`acc-03/04`、`win-acc-03/04`） |
| 4 | 哈希去重 + ≥10s 修订窗口；首存 1 条；注入时钟单测 | 完成 |
| 5 | 列表标题/更新时间、最近更新排序、选中跨重启 | 完成（`win-acc-04/13/14`） |
| 6 | 外壳 N3 rail + 224px 面板 + 单画布 + 模式切换器 | 完成（`acc-01`） |
| 7 | theme tokens、跟随系统、手动切换持久化 | 完成（`acc-10/11`、`acc-15/16`） |
| 8 | 状态与 2px 焦点环 | 调整为：default/hover/active/disabled；焦点环按产品决策移除（文档已同步） |
| 9 | 空库诚实引导 | 完成（`acc-01`、`win-acc-01`） |
| 10 | 中文文案集中 strings | 完成 |
| 11 | core 单测 + fmt/clippy/test 全绿 | 完成 |
| 12 | `build-windows.sh` 成功；两端「新建 → 输入 → 重启 → 保留」 | 完成（详见下） |

## 四、验证证据

质量门槛（WSL）：`cargo fmt --check` 通过；`cargo clippy --workspace --all-targets` 0 警告；`cargo test --workspace` **27 passed**（迁移幂等、CRUD、软删除过滤、修订去重与限频、标题推导、哈希向量、设置读写等）。

双端冒烟：

| 平台 | 结果 |
|---|---|
| WSLg | 空态 → Ctrl+N → 中文 Markdown 粘贴/输入 → 标题推导 → 强杀重启正文逐字一致、列表与选中恢复 → 主题明暗循环并持久化 → 去环/间距/图标复查（`acc-01`–`acc-16`） |
| Windows（release，MSVC） | `./scripts/build-windows.sh` 成功；干净数据目录空态 → 新建 → 输入 → 强杀重启保留（`win-acc-01`–`04`）；图标/分隔/对齐/侧栏开关复查（`win-acc-10`–`23`） |

数据库抽查（Windows 端）：`user_version=1`；item 为 UUIDv7；首存 `versions` 1 条；`settings` = `library.selected_prompt`、`rail.expanded=true`、`theme=system`；正文与粘贴内容逐字一致。`variables`、`items_fts` 为空属预期（工单 02/04 范围）。

截图位置：`C:\Users\Carlos\AppData\Local\Temp\{acc-*,win-acc-*}.png`；后期截图使用 `capture-window`（PrintWindow，不抢前台焦点）。

## 五、已知限制与未决事项

1. **未 commit**：按交接约定保持工作区改动，首次提交需一并决定 `.agents/`、`.scratch/` 的版本管理策略。
2. Windows 可执行文件在资源管理器中的图标（`.ico` 嵌入）未做，属打包/分发阶段；运行时窗口与任务栏图标已生效。
3. 标题输入框聚焦时下划线变 accent（非环状提示）暂留；如需一并去除为 2 行改动。
4. `variables`/`items_fts` 建表但不写入：变量解析与复制由工单 04 落地（语法按 ADR-0008 `{{name:默认}}`），FTS 检索由工单 02 落地。
5. 焦点环移除后键盘 Tab 无可见指示（产品决策）；后续键盘可达性（工单 07）需评估替代方案。
6. 临时截图与辅助脚本位于系统 Temp 目录，不入库。

## 六、下一步建议

按 `.scratch/slice-01-library-loop/issues/` 顺序继续：02 搜索（title/body/CJK）、03 Markdown 只读与模式切换、04 变量复制、05 置顶与排序、06 标签、07 键盘与状态收尾。每个工单完成后同样以 fmt/clippy/test + 双端冒烟闭环，并在工单追加 `## Comments`。
