# SQLite 为唯一真源，Markdown 为一等内容/交换格式

「Markdown 一等公民」按**内容格式**解释，而非磁盘文件真源：所有数据（内容、标签、变量、版本）存 SQLite；Markdown 是编辑、复制、导入导出的格式。

**Considered Options**：文件库（`.md` + front-matter）为真源、SQLite 仅做索引——对外部编辑与自有同步（git/Syncthing）友好，但需要文件监听、冲突消解、ID 稳定性与索引重建，MVP 复杂度不可接受。

**Consequences**：若未来要求「在 PromptDeck 之外直接编辑」，需要新增文件同步层；因此内容与 ID 必须保持可完整序列化为 Markdown（导出/导入），为升级留路。
