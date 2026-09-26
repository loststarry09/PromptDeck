# PromptDeck

Local-first、键盘优先的 Prompt 工作台。Rust + Slint + SQLite（bundled），单文件原生桌面应用。

当前阶段：**工程骨架 + 文档基线**，尚未实现业务功能。

## 环境要求

- Rust stable（1.98.1 已验证），组件 rustfmt / clippy
- WSL：Slint 图形依赖（见 `report/environment-setup.md` 第 4 节）
- Windows：MSVC Build Tools + Windows SDK（见 `report/environment-setup.md`）

## WSL 开发（日常主工作区）

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets
cargo test
cargo run                 # 经 WSLg 运行 GUI
```

## Windows 原生构建

```bash
./scripts/build-windows.sh                 # release
./scripts/build-windows.sh --profile dev   # debug
```

产物：`D:\build\promptdeck\target\x86_64-pc-windows-msvc\<profile>\promptdeck.exe`

## 工程结构

- `crates/promptdeck-core` — 纯 Rust：领域模型、存储（rusqlite）、Markdown/Variables 解析，无 Slint 依赖
- `crates/promptdeck-app` — Slint UI 与装配层，二进制名 `promptdeck`

## 数据目录解析优先级

1. 环境变量 `PROMPTDECK_DATA_DIR`
2. 可执行文件同级的 `portable.flag` → `.\data`
3. 平台默认：Windows `%LOCALAPPDATA%\PromptDeck`、Linux `$XDG_DATA_HOME/promptdeck`

## 文档

- `CONTEXT.md` — 领域术语表（唯一词汇来源）
- `docs/adr/` — 架构决策记录
- `docs/` — MVP 范围、模块边界、数据模型、首个开发切片
- `design/design.md` — 锁定的设计系统（Hallmark studied-DNA）
- `report/` — 环境基线与搭建报告
