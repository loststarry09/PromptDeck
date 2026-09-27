# PromptDeck

Local-first、键盘优先的 Prompt 工作台。为个人高级用户管理、组装并高效复用 prompt 而存在；单机运行、不依赖云服务。优先作者自用，同时按「陌生用户可直接使用」的标准设计，但不做多用户、同步与协作。

## Language

### 内容模型

**Library**:
PromptDeck 中用户全部 Prompt 与 Reusable Block 的集合。
_Avoid_: 仓库、vault、收藏夹

**Prompt**:
一段可复用的指令文本，是 Library 的基本单位；MVP 中唯一存在的内容类型。
_Avoid_: 模板、条目、note

**Pin**（置顶）:
Prompt 的布尔标记；置顶的 Prompt 在 Library 列表中排在未置顶者之前，且与 `updated_at` 相互独立（切换置顶不改变更新顺序）。
_Avoid_: 收藏、星标

**Reusable Block**（Block）:
可被多个 Prompt 引用的 Markdown 片段。MVP 不实现，数据模型预留。
_Avoid_: 片段、snippet、组件

**Composition**:
由若干 Reusable Block 与内联文本组装成一个 Prompt 的过程与结果。MVP 不实现。
_Avoid_: 拼接、合并

**Variable**:
Prompt 正文中待填充的占位符，写作 `{{name}}`，可带默认值。MVP 的核心能力之一。
_Avoid_: 参数、槽位、placeholder

**Version**:
一个 Prompt 在某一时刻的内容快照，用于回溯。MVP 记录数据但不提供历史 UI。
_Avoid_: 修订、历史、revision

**Tag**:
附加在 Prompt 上的横向标签，用于过滤与导航；MVP 不使用 Folder 层级。
_Avoid_: 分类、目录、文件夹

### 交互

**Single canvas**（单画布）:
PromptDeck 的编辑形态：同一画布在 Markdown 与 Source 两种模式间切换，而非并列窗格。
_Avoid_: 编辑器面板

**Source mode**:
单画布中以纯 Markdown 文本编辑的模式。MVP 中唯一可编辑的模式。
_Avoid_: 源码视图

**Markdown mode**:
单画布中渲染 Markdown 的只读模式。两种模式经左下角切换器切换。
_Avoid_: 预览、渲染视图

**Quick Launcher**:
应用内命令面板（Ctrl+K），用于搜索 Library 与执行命令。MVP 仅应用内，无系统级热键。
_Avoid_: 启动器、palette、spotlight

### 采集

**Prompt Radar**:
从外部来源（剪贴板、粘贴、目录）在本地捕获 Prompt 候选的采集机制；默认关闭、无网络、无全局键盘监听。MVP 不实现。
_Avoid_: 监控、扫描器

**Inbox**:
Prompt Radar 捕获到的候选内容等待整理入库的暂存区。MVP 不实现。
_Avoid_: 收件箱、队列
