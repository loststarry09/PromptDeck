# PromptDeck 初始化与 MVP 基础报告

执行日期：2026-09-26。范围：正式初始化 Git 与工程骨架、建立文档/ADR/CONTEXT、以 grill-with-docs 完成需求与关键设计决策、明确 MVP/模块/数据模型/首个切片、完成双端 smoke 验证。**未实现任何业务功能。**

## 一、流程（ask-matt 判定）

- 判定：有工作目录的 greenfield 项目 → 主流程从 **`/grill-with-docs`** 开始（stateful，产出 CONTEXT.md 与 ADR），而非 `/wayfinder`（范围足够清晰，无需先测绘迷雾）。
- 执行：grill-with-docs 三轮共 17 问（含推荐答案），全部决策已确认；按 `/domain-modeling` 纪律即时维护 `CONTEXT.md`。
- UI/UX：按 **hallmark** 纪律使用；设计 DNA 锁定于 `design/design.md`，映射见 `docs/design-foundations.md`。
- 工作流前置：按 `setup-matt-pocock-skills` 建立 `docs/agents/`（本地 markdown issue tracker、默认 triage labels、single-context domain docs）。

## 二、交付物

| 类别 | 位置 |
|---|---|
| Git | `.git`（main 分支，**尚无提交**）、`.gitignore`、`.gitattributes` |
| 工具链配置 | `rust-toolchain.toml`（stable + rustfmt/clippy）、`.cargo/config.toml`（msvc `/STACK:8000000`） |
| 工程骨架 | workspace：`crates/promptdeck-core`（paths/storage，无 Slint）+ `crates/promptdeck-app`（Slint 窗口 + 装配，二进制 `promptdeck`） |
| 构建 | `scripts/build-windows.sh`（rsync 到 `D:\build\promptdeck` + MSVC 原生构建） |
| 文档基线 | `CONTEXT.md`（术语表）；`docs/mvp.md`、`docs/architecture.md`、`docs/data-model.md`、`docs/design-foundations.md`、`docs/slices/01-library-loop.md` |
| ADR | `docs/adr/0001`–`0008`（技术栈、SQLite 真源、单画布、隐私、构建链路、数据目录、修订累积、变量语法） |
| Agent 配置 | `AGENTS.md`、`docs/agents/{issue-tracker,triage-labels,domain}.md` |

## 三、关键决策摘要（详见 ADR / docs）

1. 目标用户：作者自用优先，同时按陌生用户可直接使用设计；不做多用户/同步/协作。
2. **MVP = 精简纵向切片**：Library + Variables + 单画布（Source 编辑 + Markdown 只读渲染）+ FTS 搜索 + 复制 + 应用内 Quick Launcher；Blocks/Composition → v0.2，Versions UI → v0.3，Radar/Inbox → 独立里程碑。
3. 存储：**SQLite 唯一真源**，Markdown 是一等内容/交换格式；内容与 ID 保持可序列化（ADR-0002）。
4. 编辑器：Slint 无富文本编辑 → MVP 只读渲染 + Source 编辑；块级混合编辑为 MVP 后首个 UX 投资（ADR-0003）。
5. 隐私：无键盘钩子/全局监听；Radar 默认关、本地处理、不出网；系统级热键若做，必须 OS 注册（ADR-0004）。
6. 数据模型：统一 `items` + `kind`（prompt/block）、Tags-only、`{{name:默认}}` 解析式变量、FTS5 trigram、版本从 MVP 起累积。
7. 工程结构：core（纯 Rust）+ app（Slint）两 crate；中文优先 UI，字符串集中管理；主题跟随系统。
8. 首个切片：端到端 Library 回路 + 按 design.md tokens 的正确外壳（验收标准见 `docs/slices/01-library-loop.md`）。

## 四、验证证据（smoke-level）

| 验收项 | 结果 |
|---|---|
| WSL `cargo fmt --check` | 通过 |
| WSL `cargo clippy --workspace --all-targets` | 0 警告 |
| WSL `cargo test --workspace` | 6 passed（数据目录解析 4、迁移幂等 1、bundled FTS5 可用 1） |
| WSL `cargo build --workspace` + WSLg 运行 4s | 正常（timeout 124，无崩溃），WAL 落盘 |
| Windows `./scripts/build-windows.sh`（release，MSVC） | 成功，`promptdeck.exe` 约 13.6MB |
| Windows GUI 运行 5s | 正常（无崩溃），WAL 落盘 |

## 五、未决事项

1. **Git 尚无初始提交**（等确认）。
2. **许可证未定**（Q16 选择「暂不定」）：发布前需决定并补 LICENSE。
3. 块引用/Composition 语法（v0.2 设计时定）；Inbox 实体 schema 留待 v0.4。
4. 发布体系（Installer/Portable/签名/静态 CRT）按计划推迟；`crt-static` 未启用。
5. `.agents/` 为项目内 Agent skill 目录，未加入 .gitignore——首次提交时需决定是否纳入版本管理（涉及第三方 skill 的再分发许可）。

## 六、下一步建议

进入主流程下一段：**`/to-spec`（把本阶段文档收敛为 Slice 01 规格）→ `/to-tickets`（拆最终产物式工单，声明阻塞边）→ 逐个 `/implement`**。Slice 01 的验收标准已就绪，无需要 `/wayfinder` 的迷雾决策。
