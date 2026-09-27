# 阶段 01 交接报告（截至工单 03.1）

阅读对象：接手本项目的**新 Agent 对话**。目标是一份高密度上下文，使新会话**只读本文件 + 当前工单**即可理解项目并继续开发。

## 0. Source of truth 优先级（冲突时以此为准）

1. **`docs/adr/0001`–`0008`**（锁定决策）与 **`CONTEXT.md`**（唯一词汇表）。
2. **`docs/mvp.md`、`docs/data-model.md`、`docs/architecture.md`、`docs/design-foundations.md`** 与**当前切片 spec / ticket**（`.scratch/slice-01-library-loop/{spec.md,issues/*}`）。
3. 本交接摘要：只做压缩与导航，**不得**用于推翻上述文件。
4. `report/`（各工单完成报告）是“已发生了什么”的记录，但**已被 `.gitignore` 排除**，仅本地存在；后续以 ticket 的 `## Comments` 和 git 历史为准。

> 约定：领域概念一律使用 `CONTEXT.md` 词汇（Prompt / Reusable Block / Composition / Variable / Version / Tag / Single canvas / Source mode / Markdown mode / Quick Launcher / Prompt Radar / Inbox）；禁止用其 `_Avoid_` 同义词。

---

## 1. 产品定位与核心目标

- **PromptDeck**：Local-first、键盘优先的 **Prompt 工作台**。本地单机、不联网、不做多用户/同步/协作。
- 用户：**作者本人优先**（单一高级用户），但按“陌生用户可直接使用”的标准设计。
- 核心任务：把 prompt 当作可维护资产，**从可复用部件快速组装高质量 prompt 并送进目标应用**。
- **MVP 成功标准**（`docs/mvp.md`）：不打开其他工具即可完成 **新建 → 编辑（Markdown）→ 自动保存 → 搜索找回 → 填充变量 → 复制到目标应用**，且重启后数据完好。

## 2. 已确认、不可随意推翻的约束

### 产品 / UX

- UI **全中文**，文案集中在 `crates/promptdeck-app/src/strings.rs`（为 i18n 留位，但 MVP 无运行时切换）。
- **Single canvas**：同一画布在 Source（唯一可编辑）与 Markdown（只读）间切换，**非并列窗格**（ADR-0003）。
- 键盘优先：`Ctrl+N` 新建、`Ctrl+M` 切模式、`Ctrl+S` flush、`Ctrl+Shift+C` 复制最终文本、`↑/↓`/`Enter`/`Esc`、`Ctrl+K` Quick Launcher（slice 02）。
- **隐私**（ADR-0004）：禁止键盘钩子/全局输入监听；Radar 默认关、本地处理、不出网；若做系统级热键必须用 OS 注册（Windows `RegisterHotKey`）。
- 设计系统真源为 `design/design.md`（**锁定**），OKLCH 值一次性转 sRGB 写入 `ui/theme.slint`，**theme 之外禁止硬编码颜色/字体**；Hallmark 纪律：诚实文案（空状态不编造）、标题正体（禁斜体标题）、不重绘窗口装饰。
- 交互组件需 default/hover/active/disabled 状态（loading/error/success 按需）；不靠颜色单独表达状态；单手目标 ≥44×44px。
- **已确认的偏离**：2026-09-26 产品决策**移除焦点环/聚焦高亮**；文本输入（标题、搜索）保留底部 accent 下划线作聚焦提示，且输入组件**不设 hover**（Slint `TextInput` 无 hover，覆盖层会破坏光标定位）。键盘 Tab 无可见指示是已知代价。

### 技术（ADR 摘要）

| ADR | 决策 | 要点 |
|---|---|---|
| 0001 | Rust 2024 + Slint 1.x + rusqlite(bundled) | 单原生二进制；不依赖 WebView/Node/外部 DB；femtovg + software 回退，**不启用 Skia** |
| 0002 | **SQLite 唯一真源** | Markdown 是内容/交换格式而非磁盘真源；内容与 ID 必须可完整序列化为 Markdown |
| 0003 | 单画布：Source 可编辑，Markdown 只读 | Slint 无富文本编辑；块级混合编辑推迟为 MVP 后首个 UX 投资 |
| 0004 | 无全局键盘监听 | 见上 |
| 0005 | WSL 源码，NTFS 镜像 + Windows MSVC 原生构建 | 始终显式 `--target x86_64-pc-windows-msvc`，不设全局默认 target，不跨编译 |
| 0006 | 数据目录：`PROMPTDECK_DATA_DIR` → `portable.flag`(→`./data`) → 平台默认 | Windows `%LOCALAPPDATA%\PromptDeck`、Linux `$XDG_DATA_HOME/promptdeck`；安装版不写安装目录 |
| 0007 | Version 数据从 MVP 起累积，历史 UI 后置 | flush 时内容哈希变化且距上一修订 ≥10s 才落一条 |
| 0008 | Variable 语法 `{{name}}` / `{{name:默认值}}`，解析式 | `\{{` 转义；保存时解析落表；不存用户填过的值；语法是兼容契约 |

### 存储与迁移约束

- `storage::MIGRATIONS` 索引即 `PRAGMA user_version`。**首个公开发布前允许原地修改迁移 1；发布后冻结，只允许追加**。开发期改 schema 后删本地库重建即可。
- ID = UUID v7（TEXT，小写）；时间 = UTC Unix 毫秒；`body_md` 一律 Markdown 原文，渲染格式不落库；`items.deleted_at` 软删除从第一天存在。
- 迁移/时间/ID 通过注入 provider（`clock`/`id`）保证测试确定性。
- 质量门槛（每次提交）：`cargo fmt --check`、`cargo clippy --workspace --all-targets`（0 警告）、`cargo test --workspace` 全绿。

## 3. 当前架构与模块

依赖方向严格单向：`promptdeck-app → promptdeck-core → rusqlite`。**core 不得引用 Slint；app 不得直接写 SQL。**

```
crates/promptdeck-core   # 纯 Rust，无 Slint；可被未来 CLI/系统集成复用
crates/promptdeck-app    # Slint UI 与装配层；二进制名 promptdeck
```

### core 模块

| 模块 | 现状 |
|---|---|
| `paths` | 数据目录解析（ADR-0006） |
| `storage/mod.rs` | 连接、PRAGMA（WAL、`foreign_keys=ON`、`busy_timeout=5000`、`synchronous=NORMAL`）、迁移 |
| `storage/library.rs` | `Library` 仓储：items CRUD、软删除、FTS 同事务维护、versions 去重、settings |
| `model` | `Item`、`ItemKind`、`ItemSummary`、`Revision`、`derive_title`、`content_hash`(FNV-1a 64) |
| `markdown` | 块级解析（`parse`）+ 行内解析/atom 化（`parse_inlines`/`atomize_spans`/`atomize_code`） |
| `search` | `plan()` → `QueryPlan::{All,Trigram,Like}` |
| `clock` / `id` | 可注入时间与 ID provider |
| `error` | `thiserror` 类型化错误 |
| **`variables`** | **尚未创建**（工单 04） |

### app 模块

| 模块 | 职责 |
|---|---|
| `main` | 装配根：解析数据目录 → 打开并迁移数据库 → 建 `Controller` → `ui.run()` → 退出时 `flush()` |
| `controller` | core 操作 ↔ Slint 属性/回调；列表模型、选中、按条目画布模式、防抖自动保存、Markdown 渲染与选择接线 |
| `markdown` | `render()`：core 块 → `UiDocument{blocks,texts,atom_ranges}` |
| `selection` | Markdown 只读选择纯函数模型：`Document`/`Selection`/`Position`/`AtomRect`/`BlockInfo`，`hit_test`/`select_all`/`normalized`/`text` |
| `strings` | 中文文案集中管理 |
| `theme` | 主题循环/持久化装配 |
| `ui/*.slint` | `app`(窗口+KeyBinding) `shell` `library` `canvas` `markdown` `theme`(tokens) `strings` `icons` |

### 数据流 / 线程

- **MVP 单线程**：Slint 事件循环内同步调用 core（本地 SQLite + WAL，微秒级）。
- 自动保存 800ms 防抖；切换条目、退出、`Ctrl+S` 强制 flush。
- Markdown 选择：UI 覆盖层上报 atom 几何（内容坐标）→ Rust 纯函数命中测试/提取文本；`GEOMETRY_SETTLE_DELAY=80ms` 补报几何。
- 未来重活（Radar、导入导出、本地相似度）走后台线程 + `invoke_from_event_loop`。

### 错误处理

core 返回 `thiserror` 类型化错误，绝不 panic 于用户数据；app 边界映射为状态提示（状态行/横幅），**不弹阻塞对话框**。

### 外壳结构（已实现，按最终形态）

```
Window
└─ HorizontalLayout
   ├─ Side rail（N3：62px 收起 / 176px 展开，底部按钮切换并持久化；Library / Inbox 占位 / Settings）
   ├─ Context panel（224px：搜索框 + Prompt 列表 + Tags 过滤位）
   └─ Single canvas（标题 + Source 编辑 / Markdown 只读渲染 + 左下角模式切换器）
```

## 4. 已完成能力（01 / 02 / 03 / 03.1，只记结果）

| 工单 | 结果能力 | 关键落点 |
|---|---|---|
| **01** 新建/编辑/持久化 + 正确外壳与 tokens | `Ctrl+N` 新建并聚焦；标题可编辑、空时从正文首行推导（去 `#`、截 80 字）；800ms 防抖自动保存；重启正文逐字一致且列表/选中恢复；首存落 1 条 Version，之后哈希去重 + ≥10s 限频；N3 shell + 224px 面板 + 单画布 + 模式切换器；OKLCH→sRGB 的 Light/Azure + Dark/Forest，跟随系统可切换并持久化；诚实空态；SVG 图标经 `AppIcon` 按 tokens 着色 | `storage/library.rs`、`model.rs`、`controller.rs`、`ui/theme.slint` |
| **02** 搜索（标题/正文，中文可用） | 搜索框即时过滤；**≥3 字符**走 FTS5 `trigram` 双引号短语；**<3 字符**回退 `LIKE '%q%'`（转义 `\ % _`）；两条路径语义均为连续子串；FTS 与 items **同事务**维护；打开时索引行数 ≠ 在用条目数则按 rowid 重建；无结果诚实空态（含查询词） | `core/search.rs`、`storage/library.rs`、`ui/library.slint` |
| **03** Markdown 只读渲染 + `Ctrl+M` 模式切换 | 单画布互斥切换；渲染标题/粗体/斜体/列表/引用/行内代码/链接；不支持的块/行内语法**降级纯文本不崩溃**；Markdown 内不可编辑、进入即失焦锚点；模式**按条目在会话内记忆**（内存）；切换时按比例保持滚动上下文 | `core/markdown.rs`、`app/markdown.rs`、`ui/markdown.slint`、`ui/canvas.slint` |
| **03.1** Markdown 文本选择/复制 | 鼠标拖选（同段/跨行/跨段；CJK 逐字、西文按词）；`Ctrl+C` 复制**渲染文本**（跨段以 `\n` 连接，无选择不改剪贴板）；`Ctrl+A` 全选；高亮用 `Theme.selection`；仅 Markdown 模式生效，Source 原生选择不受影响；列表 marker 不进 atom | `core/markdown.rs`(atom 化)、`app/selection.rs`、`controller.rs`、`ui/markdown.slint`、`arboard` |

**已实现的键位**（`ui/app.slint` `global-keys` + `ui/canvas.slint` `mode-anchor`）：`Ctrl+N`、`Ctrl+M`、`Ctrl+C`/`Ctrl+A`（仅 Markdown）。`Ctrl+Shift+C`/`Ctrl+S`/`↑↓`/`Enter`/`Esc` 尚未接线。

**Git 状态**：`main` 分支；`16cda58` bootstrap → `0caaebc` search（**已 push**）→ `fc9e5ed` markdown 只读 → `6ab043a` 选择复制（后两者**未 push**）。工作区干净。远程：`github.com/loststarry09/PromptDeck`(public)。`report/`、`.scratch/`、`.agents/` 均被 `.gitignore` 排除。

## 5. 关键行为细节

### 数据模型（`docs/data-model.md` 为准；本文仅提要）

- Schema v1 一次建齐：`items`(统一 prompt/block + `kind` + `pinned` + `deleted_at`)、`tags`、`item_tags`、`variables`、`versions`、`settings`、`items_fts`(trigram)。
- `items` 索引 `idx_items_kind_updated`；`item_tags` `WITHOUT ROWID`；`variables` 是**派生数据**（保存时解析正文写入）。
- **当前列表排序**：`ORDER BY updated_at DESC, created_at DESC, id DESC` —— `pinned` 已存储/读回但**尚未参与排序**（工单 05 实现“置顶优先”）。
- `selected_prompt`、`theme`、`rail.expanded` 存 `settings`。
- 软删除已实现（`soft_delete` + 查询过滤），**无回收站 UI**。

### Variables（ADR-0008，**尚未实现**）

- `{{name}}` / `{{name:默认值}}`；`name` 为不含空白/`{`/`}`/`:` 的 1+ 字符；`\{{` → 字面 `{{`；同名合并、默认值取首次出现；代码块内同样解析；**不持久化用户填值**。

### Markdown 渲染

- core 独立做**块级**结构；行内交给 Slint `StyledText`（CommonMark 子集）；标题以 `**…**` 包裹实现加粗（`StyledText` 无字重），失败逐级降级。
- 降级链：不支持块 → 段落文本；`from_markdown` 失败 → `from_plain_text`（不丢内容、不崩溃）。
- 代码块无横向滚动（长行按词换行）；链接仅渲染样式，**未接线打开浏览器**。

### 搜索

- 按字面处理，无短语/布尔高级语法；每条按键一次本地查询（**无防抖**，为库量级设计）。
- `items_fts` 用 `items` 隐式 rowid 关联，`item_id` 仅作 JOIN 键；保存=删旧插新，软删除=删行。

### 选择 / 复制（Markdown 只读面）

- 无原生富文本选择：内容层叠 `TouchArea` 覆盖层 → Rust 命中测试；覆盖层 `scroll-event` 返回 `reject` 交回 `ScrollView`。
- atom 几何以内容坐标（`absolute-position − page.absolute-position`）上报，与滚动无关。

## 6. 测试与构建状态

- **测试**：`cargo test --workspace` = **109 passed**（core 89 + app 20），0 failed。core 覆盖迁移幂等、CRUD/软删除、FTS/短查询兜底、修订去重限频、标题推导、哈希向量、Markdown 块/行内/atom、选择纯函数。
- **WSL（主工作区）**：`cargo fmt --check` / `cargo check` / `cargo clippy --all-targets` / `cargo test` / `cargo run`（WSLg GUI）均通过；当前无警告。
- **Windows 原生**：`./scripts/build-windows.sh`（rsync → `D:\build\promptdeck` + `cargo.exe --target x86_64-pc-windows-msvc`）成功；产物 `D:\build\promptdeck\target\x86_64-pc-windows-msvc\<profile>\promptdeck.exe`。**动态 CRT**（依赖系统 `VCRUNTIME140.dll`），`crt-static` 推迟到发布阶段。
- MVP **不做 UI 自动化**，靠双端手工冒烟 + 双端构建门槛。冒烟脚本约定：精确匹配窗口标题 `PromptDeck`/`PromptDeck (`，必要时 `AttachThreadInput` 提升前台。

## 7. 已知限制、技术债、推迟事项

- **Markdown 只读不可编辑**是 ADR-0003 的已确认边界，不是缺陷。富文本/块级混合编辑是 MVP 后首个 UX 投资。
- Markdown 链接不可点击（未接线）；代码块长 token 可在中间断行；行内 `<u>` 支持下划线、`<font color>` 剥标签留文字、图片/未知 HTML 字面显示；选择仅限内容区（左侧留白 x<视图左沿不触发）。
- **焦点环移除**后键盘 Tab 无可见指示（产品决策，工单 07 需评估替代）。
- 搜索随输入即时查询、不参与选中；筛选状态下 `Ctrl+N` 新建的条目可能不匹配当前查询而不出现在列表（画布仍选中）——留待工单 07。
- 主题切换仅改颜色/高度层级，不动几何排版；运行时窗口/任务栏图标已生效，但 `.exe` 资源管理器图标（`.ico` 嵌入）未做（发布阶段）。
- 动态 CRT 未静态化；发布体系（Installer/Portable/签名/静态 CRT/自动更新）整体推迟。
- **许可证未定**，无 `LICENSE` 文件（发布前决定）。
- **`.scratch/`、`.agents/`、`report/` 的版本管理策略未决**（当前均 gitignore；`.agents/` 涉及第三方 skill 再分发许可）。
- **OPEN（backlog）**：Markdown 渲染 CJK 字形疑似异常（例：`区` 在暗色长文观感与邻字不一致）——**未定位，用户要求暂停并记录**；假设涉及逐字字体回退或 atom `preferred-width` 取整裁边，验证方法见 `.scratch/backlog.md`。

## 8. 当前 MVP 边界：尚未实现（Slice 01 剩余 + 明确不做）

**Slice 01 尚未实现（04–07）**：

- **04** 变量解析与 `Ctrl+Shift+C` 复制最终文本（含 `\{{`、多行/空默认值、去掉并落 `variables` 表、非阻塞反馈）。
- **05** 收藏/置顶切换 UI + “置顶优先，其余按最近更新”排序（`pinned` 已入库，缺 UI 与排序）。
- **06** Tags 编辑与过滤（增删标签、chip 过滤、与搜索叠加、大小写不敏感唯一）。
- **07** 键盘工作流与状态收口（`↑↓`/`Enter`/`Esc`、`Ctrl+S`、“保存中/已保存/失败”可见、失败不丢内容）+ Slice 01 完整 DoD 验收。

**明确不做（MVP 外，勿顺手实现）**：

- Reusable Block 与 Composition Engine（v0.2；数据模型已预留 `kind='block'` 与引用位）
- Version 历史 UI（v0.3；数据已在积累）
- Prompt Radar + Inbox（独立里程碑）
- Quick Launcher 面板（**slice 02**）
- 变量填值表单与用户填值持久化（slice 02）
- 删除/回收站 UI、导入/导出、设置页（主题开关除外）、Folder 层级、i18n 运行时切换、插件/脚本、云同步、发布体系

## 9. 下一步：工单 04 起点与不得越界

**起点文件**：`.scratch/slice-01-library-loop/issues/04-variables-copy-resolved.md`（实现契约见 `spec.md` 的 “复制/Variables” 段与 ADR-0008 / `docs/data-model.md`）。

**04 要做的**（工单原文）：

- 保存时解析变量落 `variables` 表：同名合并一条、默认值取首次出现；删除条目级联清理。
- `\{{` → 字面 `{{`；支持多行/空默认值。
- `Ctrl+Shift+C` 复制“默认值已填充”的最终文本；无默认值的变量保持 `{{name}}` 原样。
- 复制成功**轻量非阻塞反馈**；**普通 `Ctrl+C` 的选择语义不变**。
- 剪贴板用 `arboard` 在 **app 层**实现，不引入 WebView/新运行时。
- core 表驱动单测；WSLg + Windows 冒烟（复制后粘贴到外部编辑器验证替换）。

**不得越界**：

- 不实现变量填值表单、值持久化、类型/描述/作用域（slice 02 / post-MVP）。
- 不改 ADR-0008 语法契约；正文中的字面 `{{` 必须靠 `\{{`。
- 不顺手做 05 置顶/排序、06 标签、07 键盘收口；不引入 WebView 或新渲染依赖。
- 不修改迁移 1 之外的历史；如需改 schema 仍属“发布前允许原地改迁移 1”的窗口。
- 不在 core 引用 Slint，不在 app 直接写 SQL；颜色/字体只能来自 `ui/theme.slint`。
- 门槛与约定：`fmt`/`clippy`(0 警告)/`test` 全绿 + 双端冒烟；完成后在工单追加 `## Comments`；未获明确指示**不要 commit/push**。

## 10. 新 Agent 继续阅读的文件索引

**必读（按序）**：

1. `CONTEXT.md` — 领域词汇，输出命名必须对齐。
2. `docs/adr/0001`–`0008` — 锁定决策（尤其 0002/0003/0006/0007/0008）。
3. `docs/mvp.md` — 范围与里程碑。
4. `docs/data-model.md` — schema、Variables 规则、修订策略。
5. `docs/architecture.md` — 模块边界与测试策略。
6. `.scratch/slice-01-library-loop/spec.md` + 当前 ticket（04）。
7. 本文件（`docs/stage-01-handoff.md`）。

**按需**：

- `docs/design-foundations.md` + `design/design.md`（改 UI 时；design.md 为锁定真源）。
- `docs/slices/01-library-loop.md` — 切片 DoD 验收清单。
- `.scratch/slice-01-library-loop/issues/{05,06,07}.md` — 后续工单。
- `.scratch/backlog.md` — CJK 字形 OPEN 项。
- `README.md` — 命令与数据结构。
- 代码入口：`crates/promptdeck-core/src/{model,storage/library,markdown,search}.rs`、`crates/promptdeck-app/src/{controller,markdown,selection}.rs`、`crates/promptdeck-app/ui/{app,canvas,markdown,theme}.slint`。
- `report/*.md`（本地、gitignore）— 历史完成报告，仅在需要细节时查；**不作为决策依据**。

**建议 skills**：继续实现用 `implement`（配合 `tdd`，core 走真实内存 SQLite 主 seam）；完成后用 `code-review` 做 Standards/Spec 双轴审查；涉及新决策时用 `domain-modeling` / `grill-with-docs` 维护 `CONTEXT.md` 与 ADR；跨会话状态用 `handoff`；不确定流程时用 `ask-matt`。
