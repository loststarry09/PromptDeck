# 单画布 MVP：Source 可编辑，Markdown 只读渲染

Slint 只提供纯文本 `TextEdit` 与只读 `StyledText`（CommonMark 子集），没有富文本编辑。MVP 因此定义：Source mode 是唯一可编辑面，Markdown mode 是同一画布上的只读渲染，左下角切换；Typora 式「块级混合编辑」（聚焦块可编辑、失焦块渲染）推迟为 MVP 之后第一个 UX 投资。

**Considered Options**：自研富文本编辑器（MVP 体量不可接受）、嵌入 WebView（违背 ADR-0001）。

**Consequences**：Markdown mode 在 MVP 内不可编辑，编辑必须切回 Source；切换器与渲染层按最终形态先建，后续升级为混合编辑时不改外壳。
