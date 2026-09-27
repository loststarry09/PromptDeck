# PromptDeck — Slice 01 最终交接报告

阅读对象：接手本项目的**新 Agent 对话**。本文件已压缩至「只读本文件 + 当前任务」即可恢复上下文。目标是一份高密度、无过时表述的交接摘要。

> 状态：**Slice 01（工单 01–07）已完成并双端验收**。当前阶段**暂停**，等待作者真实使用反馈（见 §9）。

## 0. Source of truth 优先级（冲突时以此为准）

1. **`docs/adr/0001`–`0009`**（锁定决策）与 **`CONTEXT.md`**（唯一词汇表）。
2. **`docs/mvp.md`、`docs/data-model.md`、`docs/architecture.md`、`docs/design-foundations.md`** 与**当前切片 spec / ticket**（`.scratch/slice-01-library-loop/{spec.md,issues/*}`）。
3. 本交接摘要：只做压缩与导航，**不得**用于推翻上述文件。
4. `report/`（各工单完成报告）是「已发生了什么」的记录，**冷存储**——已被 `.gitignore` 排除、仅本地存在，仅在需要细节时查，**不作为决策依据**；后续以 ticket 的 `## Comments` 和 git 历史为准。

> 阅读规则（默认必读 vs 按需读取）见 §10。

> 约定：领域概念一律使用 `CONTEXT.md` 词汇（Prompt / Reusable Block / Composition / Variable / Version / Tag / Single canvas / Source mode / Markdown mode / Quick Launcher / Prompt Radar / Inbox）；禁止用其 `_Avoid_` 同义词。

---

## 1. 产品定位与核心目标

- **PromptDeck**：Local-first、键盘优先的 **Prompt 工作台**。本地单机、不联网、不做多用户/同步/协作。
- 用户：**作者本人优先**（单一高级用户），但按「陌生用户可直接使用」的标准设计。
- 核心任务：把 prompt 当作可维护资产，**从可复用部件快速组装高质量 prompt 并送进目标应用**。
- **MVP 成功标准**（`docs/mvp.md`）：不打开其他工具即可完成 **新建 → 编辑（Markdown）→ 自动保存 → 搜索找回 → 填充变量 → 复制到目标应用**，且重启后数据完好。**该回路已在 Slice 01 端到端跑通**。

## 2. 已确认、不可随意推翻的约束

### 产品 / UX

- UI **全中文**，文案集中在 `crates/promptdeck-app/src/strings.rs`（为 i18n 留位，但 MVP 无运行时切换）。
- **Single canvas**：同一画布在 Source（唯一可编辑）与 Markdown（只读）间切换，**非并列窗格**（ADR-0003）。
- **隐私**（ADR-0004）：禁止键盘钩子/全局输入监听；Radar 默认关、本地处理、不出网；若做系统级热键必须用 OS 注册（Windows `RegisterHotKey`）。
- 设计系统真源为 `design/design.md`（**锁定**），OKLCH 值一次性转 sRGB 写入 `ui/theme.slint`，**theme 之外禁止硬编码颜色/字体**；Hallmark 纪律：诚实文案、标题正体（禁斜体标题）、不重绘窗口装饰。
- 交互组件具备 default/hover/active 状态（focus 见 §5）；不靠颜色单独表达状态；单手目标 ≥44×44px。
- **焦点可见性（2026-09-27 修订）**：2026-09-26 曾决策「完全移除焦点环」，导致键盘 Tab 无可见指示，构成可达性缺陷；工单 07 改为 **keyboard-only focus-visible**：非输入元素仅在键盘 Tab 导航聚焦时显示 2px accent 环（`ui/focusring.slint`），指针点击不显示；文本输入（标题、搜索、标签）保留底部 accent 下划线，且输入组件不设 hover。accent 填充的主 CTA（「新建」）焦点环改用 `paper`。详见 `docs/design-foundations.md:34`。

### 技术（ADR 摘要）

| ADR | 决策 | 要点 |
|---|---|---|
| 0001 | Rust 2024 + Slint 1.x + rusqlite(bundled) | 单原生二进制；不依赖 WebView/Node/外部 DB；femtovg + software 回退，**不启用 Skia** |
| 0002 | **SQLite 唯一真源** | Markdown 是内容/交换格式而非磁盘真源；内容与 ID 必须可完整序列化为 Markdown |
| 0003 | 单画布：Source 可编辑，Markdown 只读 | Slint 无富文本编辑；块级混合编辑推迟为 MVP 后首个 UX 投资 |
| 0004 | 无全局键盘监听 | 见上 |
| 0005 | WSL 源码，NTFS 镜像 + Windows MSVC 原生构建 | 始终显式 `--target x86_64-pc-windows-msvc`，不跨编译 |
| 0006 | 数据目录：`PROMPTDECK_DATA_DIR` → `portable.flag`(→`./data`) → 平台默认 | Windows `%LOCALAPPDATA%\PromptDeck`、Linux `$XDG_DATA_HOME/promptdeck` |
| 0007 | Version 数据从 MVP 起累积，历史 UI 后置 | flush 时内容哈希变化且距上一修订 ≥10s 才落一条；新建首存立即一条 |
| 0008 | Variable 语法 `{{name}}` / `{{name:默认值}}`，解析式 | `\{{` 转义；保存时解析落表；不存用户填过的值；语法是兼容契约 |

### 存储与迁移约束

- `storage::MIGRATIONS` 索引即 `PRAGMA user_version`。**首个公开发布前允许原地修改迁移 1；发布后冻结，只允许追加**。开发期改 schema 后删本地库重建即可。
- ID = UUID v7（TEXT，小写）；时间 = UTC Unix 毫秒；`body_md` 一律 Markdown 原文，渲染格式不落库；`items.deleted_at` 软删除从第一天存在（无回收站 UI）。
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
| `storage/library.rs` | `Library` 仓储：items CRUD、软删除、FTS 同事务维护、versions 去重、variables 派生、tags、settings |
| `model` | `Item`、`ItemKind`、`ItemSummary`、`Tag`、`VariableDef`、`Revision`、`derive_title`、`content_hash`(FNV-1a 64) |
| `markdown` | 块级解析（`parse`）+ 行内解析/atom 化（`parse_inlines`/`atomize_spans`/`atomize_code`） |
| `search` | `plan()` → `QueryPlan::{All,Trigram,Like}` |
| `variables` | 解析变量定义 + 最终文本替换 |
| `clock` / `id` | 可注入时间与 ID provider |
| `error` | `thiserror` 类型化错误 |

未来：`composition`（v0.2 块组装）、`capture`（Radar 采集适配器）。

### app 模块

| 模块 | 职责 |
|---|---|
| `main` | 装配根：解析数据目录 → 打开并迁移数据库 → 建 `Controller` → `ui.run()` → 退出时 `flush()` |
| `controller` | core 操作 ↔ Slint 属性/回调；列表模型、选中、按条目画布模式、防抖自动保存、保存状态、Markdown 渲染与选择接线 |
| `markdown` | `render()`：core 块 → `UiDocument{blocks,texts,atom_ranges}`；行内样式以 **atom 属性**表达（**不使用 `StyledText`**） |
| `selection` | Markdown 只读选择纯函数模型：命中测试/规范化/文本提取 |
| `listnav` | 列表键盘导航纯函数（`stepped_index`，`↑`/`↓` 越界收敛） |
| `strings` | 中文文案集中管理，为 i18n 留位 |
| `theme` | 主题循环/持久化装配 |
| `ui/*.slint` | `app`(窗口+KeyBinding) `shell` `library` `canvas` `markdown` `focusring` `strings`(UI 文案) `icons` `theme`(tokens) |

### 数据流 / 线程

- **MVP 单线程**：Slint 事件循环内同步调用 core（本地 SQLite + WAL，微秒级）。
- 自动保存 800ms 防抖；切换条目、退出、`Ctrl+S` 强制 flush。
- Markdown 选择：UI 覆盖层上报 atom 几何（内容坐标）→ Rust 纯函数命中测试/提取文本；`GEOMETRY_SETTLE_DELAY=80ms` 补报几何。
- 未来重活（Radar、导入导出、本地相似度）走后台线程 + `invoke_from_event_loop`。

### 错误处理

core 返回 `thiserror` 类型化错误，绝不 panic 于用户数据；app 边界映射为状态提示（状态行/标签内联错误），**不弹阻塞对话框**。保存失败会 `eprintln!` 记录原因。

### 外壳结构（已实现，按最终形态）

```
Window
└─ HorizontalLayout
   ├─ Side rail（N3：62px 收起 / 176px 展开，底部按钮切换并持久化；Library / Inbox 占位 / Settings）
   ├─ Context panel（224px：搜索框 + Prompt 列表 + Tags 过滤位）
   └─ Single canvas（标题 + Source 编辑 / Markdown 只读渲染 + 左下角模式切换器 + 保存状态文字）
```

## 4. Slice 01 最终能力（工单 01–07，只记结果）

| 工单 | commit | 结果能力 |
|---|---|---|
| **01** 外壳 + 新建/编辑/持久化 | `16cda58` | `Ctrl+N` 新建并聚焦；标题可编辑、空时从正文首行推导（去 `#`、截 80 字）；800ms 防抖自动保存；重启正文逐字一致且列表/选中恢复；首存落 1 条 Version；N3 shell + 224px 面板 + 单画布 + 模式切换器；OKLCH→sRGB 的 Light/Azure + Dark/Forest，跟随系统可切换并持久化；诚实空态 |
| **02** 搜索 | `0caaebc` | 标题/正文即时过滤；**≥3 字符**走 FTS5 `trigram`；**<3 字符**回退 `LIKE '%q%'`（转义 `\ % _`）；中文子串可用；FTS 与 items 同事务维护；打开时索引行数 ≠ 在用条目数则按 rowid 重建；无结果诚实空态 |
| **03** Markdown 只读 + `Ctrl+M` | `fc9e5ed` | 单画布互斥切换；渲染标题/粗体/斜体/列表/引用/行内代码/链接；不支持的块/行内语法降级纯文本不崩溃；Markdown 内不可编辑；模式按条目在会话内记忆（内存）；切换保持滚动上下文 |
| **03.1** Markdown 选择/复制 | `6ab043a` | 鼠标拖选（同段/跨行/跨段；CJK 逐字、西文按词）；`Ctrl+C` 复制渲染文本（无选择不改剪贴板）；`Ctrl+A` 全选；高亮用 `Theme.selection`；仅 Markdown 模式生效 |
| **04** Variables + 复制最终文本 | `851367b` | 保存时解析 `{{name}}`/`{{name:默认值}}` 落 `variables` 表（同名合并、默认值取首次）；`\{{` → 字面 `{{`；`Ctrl+Shift+C` 复制默认值已填充的最终文本（无默认值保持 `{{name}}`）；成功轻量提示「已复制」；普通 `Ctrl+C` 语义不变 |
| **05** Pin（置顶） | `9f78f61` | 画布置顶开关 UI；排序「置顶优先，其余按 `updated_at DESC, created_at DESC, id DESC`」；切换置顶**不改** `updated_at` |
| **06** Tags | `926c61e` | 画布增删标签；列表行展示标签 chip（最多 3 个，其余「+N」）；点击 chip 过滤列表、可与搜索叠加；大小写不敏感唯一；空/重复标签内联提示；标签不再被使用时从词表消失 |
| **07** 键盘工作流 + 状态收口 | `6a6e8a8` | `↑`/`↓` 移动列表选择（焦点留在搜索框）、`Enter` 打开、`Esc` 画布退回/搜索清空、`Ctrl+S` 强制 flush；保存状态「保存中…/已保存/失败」；失败不丢内容并中止切换/新建；`Ctrl+N` 清空搜索与标签筛选；选中行滚动跟随；keyboard-only focus-visible 焦点环；Slice 01 完整 DoD 双端验收 |

> **Preview 收口（2026-09-27，本任务的追加项）**：Slice 01 封板后追加一次**发布收口**——版本统一 `0.1.0-preview.1`、Windows exe 嵌入图标/版本资源、静态 CRT、Inno Setup 安装包、GitHub Pre-release 分发（详见 §6）。**不引入任何新功能，不开启 Slice 02。**

## 5. 最终行为参考

### 快捷键（全部已接线）

| 快捷键 | 作用 |
|---|---|
| `Ctrl+N` | 新建 Prompt；落盘当前编辑 → 清空搜索与标签筛选 → 选中新条目并聚焦编辑 |
| `Ctrl+M` | 切换 Source / Markdown 模式（等价于左下角切换器） |
| `Ctrl+S` | 立即 flush；即使无改动也给出一次「已保存」确认 |
| `Ctrl+Shift+C` | 复制默认值已填充的最终文本（全局 capture，优先于输入；`Ctrl+C` 不受影响） |
| `↑` / `↓` | 在当前（筛选后的）列表移动选中；焦点留在搜索框，不触发切换 |
| `Enter` | 打开当前选中项，焦点进入画布（Markdown 模式下落在选择锚点） |
| `Esc` | 画布内（Source 编辑器/标题/Markdown）→ 焦点回到搜索框；搜索框内 → 清空搜索词 |
| `Ctrl+C` / `Ctrl+A` | **仅 Markdown 模式**：复制渲染选择 / 全选 |
| `Tab` | 在非输入控件间移动焦点（键盘可见焦点环）；`Enter`/`Space` 激活 |

Quick Launcher（`Ctrl+K`）**未实现**（Slice 02 或更后）。

### 保存状态

- 状态：`Idle`(0，无选中，不显示) / `Saving`(1，`保存中…`) / `Saved`(2，`已保存`) / `Failed`(3，`保存失败，按 Ctrl+S 重试`，加粗且 `eprintln` 记录原因)。
- 编辑即 `Saving`；800ms 防抖后 flush：成功 `Saved`，失败 `Failed` 且保持 `dirty`。
- **失败不丢内容**：`flush()` 返回 `bool`；`select_prompt`/`new_prompt` 在失败时中止，保留画布文本与选中；`Ctrl+S` 可在原因解除后重试成功。
- Version 去重与 ≥10s 窗口在自动保存与手动 flush 下一致。

### Markdown 渲染

- core 独立做**块级**：`Heading`(level)、`Paragraph`、`ListItem`(ordered/unordered、indent、marker)、`Quote`、`Code`、`Rule`。
- **行内**解析为带样式 **atom**（strong / emphasis / code / link / strike / underline）；UI 逐 atom 渲染并据此做选择/命中测试（**不依赖 `StyledText`**）。链接仅渲染样式，**不打开浏览器**。
- 降级：不支持的 HTML/图片等行内语法按字面文本显示，不崩溃。
- 只读；代码块长行按词换行、无横向滚动；atom `preferred-width` 取整可能导致个别 CJK 字形观感不一致（OPEN，见 §7）。

### 搜索

- 范围标题 + 正文；按字面处理，无短语/布尔高级语法。
- 每条按键一次本地查询（**无防抖**，为库量级设计）；**不改变选中**（画布仍显示当前条目）。
- ≥3 字符走 FTS5 `trigram`；<3 字符回退 `LIKE`。空态诚实：无结果（含查询词）/ 该标签下暂无 / 空库。

### Variables（ADR-0008）

- `{{name}}` / `{{name:默认值}}`；`name` 为不含空白/`{`/`}`/`:` 的 1+ 字符；`\{{` → 字面 `{{`；同名合并、默认值取首次出现；代码块内同样解析。
- 保存时解析落 `variables` 表（**派生数据**，随条目级联清理）；**不持久化用户填值**。
- `Ctrl+Shift+C`：复制默认值已填充文本；无默认值变量保持 `{{name}}` 原样。

### Pin（置顶）

- 画布标题行的置顶开关（`置顶` / `已置顶`）；置顶优先，其余按最近更新；切换置顶**不改变更新顺序**（不动 `updated_at`）；持久化。

### Tags

- 画布底部标签输入：回车添加、chip 点 × 删除；空白/重复（大小写不敏感）内联拒绝。
- 列表行展示最多 3 个标签 chip + 「+N」；点击 chip 切换过滤（再次点击或「清除」恢复全量），可与搜索叠加。
- 标签过滤是**会话内状态**（`active_tag` 在内存，不持久化）；`Ctrl+N` 会清空它。

### 主题 / 外壳

- Light/Azure 与 Dark/Forest；默认跟随系统，可手动切换并持久化在 `settings`；切换只改颜色与高度层级，不动几何排版。
- Side rail 62px↔176px，持久化展开状态。

### 焦点（keyboard-only focus-visible）

- 非输入元素（按钮、分段切换器、标签 chip）通过 `focus-gained(reason)` 判断 `FocusReason.tab-navigation`：**仅键盘 Tab 聚焦时**显示 2px accent 环（`ui/focusring.slint`），指针点击不显示（模拟 `:focus-visible`）。
- 文本输入保留底部 accent 下划线（输入组件不设 hover，避免破坏光标定位）。
- 列表行**不是** Tab 停靠点；列表选择统一走搜索框 `↑`/`↓`，避免无指示的焦点陷阱。
- Canvas 焦点只由显式请求驱动（打开/新建/用户切模式）；`↑`/`↓` 导航不抢走搜索框焦点。

## 6. 最终测试与构建基线

- **测试**：`cargo test --workspace` = **144 passed**（core 119 + app 25），0 failed。core 覆盖迁移幂等、CRUD/软删除、FTS/短查询兜底、变量解析/替换、修订去重限频、标题推导、哈希向量、Markdown 块/行内/atom；app 覆盖 `markdown::render` 映射与 `listnav::stepped_index`。
- **WSL（主工作区）**：`cargo fmt --check` / `cargo clippy --workspace --all-targets`（0 警告）/ `cargo test --workspace` / `cargo run`（WSLg GUI）均通过。
- **Windows 原生**：`./scripts/build-windows.sh` 成功；产物 `D:\build\promptdeck\target\x86_64-pc-windows-msvc\release\promptdeck.exe`。**静态 CRT**（`.cargo/config.toml` 中 `x86_64-pc-windows-msvc` 加 `+crt-static`），exe 不再依赖 `VCRUNTIME140.dll`；导入表仅系统 DLL。
- **Windows 安装包（Preview）**：`./scripts/build-windows-installer.sh` → `D:\build\promptdeck\dist\PromptDeck-0.1.0-preview.1-windows-x64-setup.exe`（含 `.sha256`）。技术方案见 **ADR-0009**：Inno Setup 6、每用户安装（无 UAC、可选安装目录）、开始菜单快捷方式、可靠卸载。
- **图标 / 版本资源**：`ui/assets/app-icon.png` → 多尺寸 `ui/assets/app.ico`，经 `crates/promptdeck-app/app.rc` + `embed-resource` 在 Windows 构建时嵌入 exe（版本 `0.1.0-preview.1`）。运行时窗口图标仍由 `ui/app.slint` 的 `icon:` 提供。
- **手工冒烟**：MVP 不做 UI 自动化；WSLg + Windows 各一轮覆盖 `↑`/`↓`/`Enter`/`Esc`/`Ctrl+S`/`Ctrl+N`/`Ctrl+M`/`Ctrl+Shift+C`、保存三态、复制解析、重启持久化。WSLg 运行提示：`SLINT_BACKEND=software` 且清 `WAYLAND_DISPLAY`（X11）；截屏需从 Windows 侧抓取（x11grab 全黑）。

## 7. 已知限制、技术债、backlog

- **Markdown 只读不可编辑**是 ADR-0003 的已确认边界，不是缺陷。富文本/块级混合编辑是 MVP 后首个 UX 投资。
- Markdown 链接不可点击；代码块长 token 可在中间断行；行内 `<u>` 支持下划线、`<font color>` 剥标签留文字、图片/未知 HTML 字面显示。
- **筛选后无可视选中时 `Enter` 仍打开画布当前条目**（可能不在列表内）；如需打开首个结果先按 `↓`。判断为可接受的边界语义，未改。
- **选中即显示「已保存」**（加载既有条目时）：表示「该条目已持久化」，非本次会话保存动作。
- 列表行最多显示 3 个标签 chip；标签过滤不持久化。
- **OPEN（backlog）**：Markdown 渲染个别 CJK 字形观感不一致（例 `区` 在暗色长文）——**未定位**，假设涉及逐字字体回退或 atom `preferred-width` 取整裁边，验证方法见 `.scratch/backlog.md`。
- 焦点块在多个 Slint 组件中重复（`FocusScope` 的 keyboard-focus 模板 ×6）、`save-state`/`selected-index` 以 int 跨 FFI、`LibraryPanel.row-height` 62px 与 `LibraryRow` 高度重复——均为评审记录的**判断项**，未在收口阶段重构。
- **Preview 发布状态**：Windows x64 安装包已提供（ADR-0009）；**未代码签名** → Windows SmartScreen 可能提示「未知发布者」，属 Preview 已知限制，不尝试绕过；**无自动更新、无 Portable、无商店发布**。静态 CRT 已完成（exe 无 VC++ 运行库依赖）。
- **许可证未定**，无 `LICENSE` 文件（发布前决定）。
- **`.scratch/`、`.agents/`、`report/` 的版本管理策略未决**（当前均 gitignore）。

## 8. 尚未实现（明确不做，勿顺手实现）

- **切片 02 及以后**：Quick Launcher 命令面板（`Ctrl+K`）、变量填值表单与值持久化。
- **MVP 外**：Reusable Block 与 Composition Engine（v0.2）、Version 历史 UI（v0.3）、Prompt Radar + Inbox、系统级全局热键。
- 删除/回收站 UI、导入/导出、设置页、Folder 层级、i18n 运行时切换、插件/脚本、云同步、发布体系。

## 9. 下一阶段：暂停，等待真实使用反馈

**当前阶段暂停。** 首个公开 Preview（`v0.1.0-preview.1`，Windows x64 安装包）已随 Slice 01 收口发布；不制定 Slice 02 实施计划。作者将实际日用一段时间；新 Agent 的默认动作是：**只读本文件 + 当前任务**，先理解现状，不主动开启新功能。待反馈后，再由作者决定方向（可能是 Quick Launcher / 变量填值体验，也可能是优先修复日用中暴露的问题）。

## 10. 阅读规则（新 Agent 默认动作）、文件索引与建议 skills

### 默认必读

仅以下两项，不要在默认情况下额外加载其他文档：

1. 本文件（`docs/stage-01-handoff.md`）。
2. 当前任务 / 当前 ticket。

### 按需读取

以下内容默认**不**主动加载，仅在当前任务确实涉及对应细节、发生文档冲突、或准备修改既有决策时才读取：

- `CONTEXT.md` — 领域词汇（输出命名需对齐时才读）。
- `docs/adr/` — 锁定决策（准备改决策前读对应 ADR；尤其 0002/0003/0006/0007/0008/0009）。
- `docs/mvp.md` — 范围与里程碑。
- `docs/data-model.md` — schema、Variables 规则、修订策略。
- `docs/architecture.md` — 模块边界与测试策略。
- `docs/design-foundations.md` + `design/design.md` — 改 UI 时（design.md 为锁定真源）。
- `docs/slices/` — 切片 DoD（如 `01-library-loop.md`，已完成）。
- `.scratch/` — 当前 spec / ticket / backlog（如 CJK 字形 OPEN 项：`.scratch/backlog.md`）。
- `report/*.md`（本地、gitignore）— **冷存储**：历史完成报告，仅在需要具体细节时查，**不作为决策依据**。
- `README.md` — 能力、快捷键、运行/构建、安装、数据目录。
- 代码入口：`crates/promptdeck-core/src/{model,storage/library,markdown,search,variables}.rs`、`crates/promptdeck-app/src/{controller,markdown,selection,listnav}.rs`、`crates/promptdeck-app/ui/{app,canvas,library,shell,markdown,focusring,theme}.slint`。
- 打包入口：`packaging/windows/promptdeck.iss`、`crates/promptdeck-app/app.rc`、`scripts/build-windows-installer.sh`。

### Source of Truth 优先级（冲突时）

**ADR / `CONTEXT.md` / 当前正式 spec 与 docs 的权威性高于本交接摘要。** 本文件只是默认上下文的压缩入口，**不拥有推翻正式决策的权限**；发现冲突时以 ADR / 正式文档为准，并顺手修正本文件。

**建议 skills**：实现新功能用 `implement`（配合 `tdd`，core 走真实内存 SQLite 主 seam）；完成后用 `code-review` 做 Standards/Spec 双轴审查；涉及新决策时用 `domain-modeling` / `grill-with-docs` 维护 `CONTEXT.md` 与 ADR；跨会话状态用 `handoff`；不确定流程时用 `ask-matt`；涉及 UI 修复用 `hallmark`（DNA 参考 `design/design.md`）。
