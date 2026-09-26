# AGENTS.md

PromptDeck：Local-first、键盘优先的 Prompt 工作台。Rust + Slint + SQLite（bundled）。

## Before exploring

- 读 `CONTEXT.md`（领域术语表，输出中命名概念时使用其中词汇）
- 读与工作区域相关的 `docs/adr/` 决策
- 读 `docs/agents/domain.md` 了解文档消费规则

## 常用命令

WSL（主要开发环境）：

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets
cargo test
```

Windows 原生构建（WSL 下调用）：

```bash
./scripts/build-windows.sh
```

不要在 WSL 中对 Windows 目标直接构建；`scripts/build-windows.sh` 已封装 rsync 到 `D:\build\promptdeck` + `cargo.exe --target x86_64-pc-windows-msvc`。

## 工程结构

- `crates/promptdeck-core` — 领域模型、存储、Markdown/Variables 解析（无 Slint）
- `crates/promptdeck-app` — Slint UI 与装配层

架构与数据模型见 `docs/architecture.md`、`docs/data-model.md`。

## Agent skills

### Issue tracker

Issues 与 spec 以本地 markdown 形式存放在 `.scratch/<feature>/`。See `docs/agents/issue-tracker.md`.

### Triage labels

使用五个规范 label 的默认值。See `docs/agents/triage-labels.md`.

### Domain docs

Single-context：根目录 `CONTEXT.md` + `docs/adr/`。See `docs/agents/domain.md`.
