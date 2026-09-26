# 技术栈：Rust + Slint + SQLite（bundled），单原生二进制

PromptDeck 要求本地优先、轻量、高性能且可 Portable 分发，因此选择 Rust 2024 + Slint 1.x（femtovg 渲染、software 回退）+ rusqlite（bundled SQLite 编进二进制），产物不依赖 WebView、Node 或外部数据库。

**Considered Options**：Tauri/Electron（WebView 运行时依赖、体积与内存开销大，且 Markdown 富渲染会依赖 Web 技术栈）被拒。

**Consequences**：UI 必须用 Slint DSL 表达，富文本编辑能力受限（见 ADR-0003）；Windows 产物默认动态 CRT，`crt-static` 留待发布阶段。
