# PromptDeck

Local-first、键盘优先的 **Prompt 工作台**。把 prompt 当作可维护的资产：本地管理、Markdown 编辑、变量填充、一键复制到目标应用。Rust + Slint + SQLite（bundled），编译为单个原生桌面可执行文件，不依赖 WebView / Node / 外部数据库，不联网。

当前状态：**Slice 01（Library 回路）已完成**，首个公开测试版为 **`v0.1.0-preview.1`（Windows x64 安装包）**。

> **Preview 测试版**，不代表稳定正式版：可能存在 UI / Markdown / 安装体验问题，且**未代码签名**。遇到问题请到 GitHub Issues 反馈。

## 当前已实现能力

- **Library**：Prompt 的列表、选中、置顶（Pin）；置顶优先、其余按最近更新排序。
- **Source / Markdown 单画布**：Source 模式纯 Markdown 编辑（唯一可编辑面）；`Ctrl+M` 或左下角切换器在 Markdown 只读渲染间切换（渲染标题、粗体、斜体、列表、引用、行内代码、链接；不支持的语法降级为纯文本，不崩溃）。
- **自动保存**：编辑约 800ms 防抖后落盘；切换条目、退出、`Ctrl+S` 强制落盘。画布底部显示 `保存中… / 已保存 / 保存失败，按 Ctrl+S 重试`。保存失败时**保留当前编辑内容**，并阻止会丢内容的切换/新建。
- **搜索**：按标题与正文即时过滤；中文子串可用（FTS5 trigram，<3 字符回退 `LIKE`）。
- **Variables**：正文支持 `{{name}}` / `{{name:默认值}}`；`Ctrl+Shift+C` 复制已用默认值填充的最终文本，无默认值的变量保持 `{{name}}` 原样，`\{{` 输出字面 `{{`。
- **Tags**：给 Prompt 增删标签；点击标签 chip 过滤列表，可与搜索叠加。
- **Markdown 选择/复制**：Markdown 模式下可鼠标拖选，`Ctrl+C` 复制渲染文本、`Ctrl+A` 全选。
- **主题**：Light/Azure 与 Dark/Forest，默认跟随系统，可手动切换并持久化。
- **键盘可达**：`↑`/`↓` 移动列表选择、`Enter` 打开、`Esc` 退回搜索/清除搜索；非输入控件支持键盘 `Tab` 聚焦（可见焦点环）与 `Enter`/`Space` 激活。
- **数据完好**：重启后内容、列表与选中状态保持。

## 安装（Windows x64）

从 GitHub Releases 下载最新 Preview 安装包并运行：

- 页面：<https://github.com/loststarry09/PromptDeck/releases>
- 文件：`PromptDeck-0.1.0-preview.1-windows-x64-setup.exe`（同目录附 `.sha256` 校验）

安装为**每用户安装**（无需管理员权限），可自选安装目录；安装后从开始菜单启动，可在「设置 → 应用」中卸载。**卸载不会删除你的 Prompt 数据**（数据在用户数据目录，见下文）。

安装包**未代码签名**，Windows SmartScreen 可能提示「未知发布者 / 更多信息」，属 Preview 已知限制。

普通用户**无需**安装 Rust / Cargo / WSL / Node / Python / SQLite；exe 为静态 CRT、自带 SQLite，开箱即可运行。

## 从源码运行（开发者）

WSL / Linux（日常主工作区，需图形环境如 WSLg）：

```bash
cargo run
```

Windows 原生：见下方「Windows 原生构建」，运行生成的 `promptdeck.exe`。

## 常用快捷键

| 快捷键 | 作用 |
|---|---|
| `Ctrl+N` | 新建 Prompt（清空搜索/标签筛选，聚焦编辑） |
| `Ctrl+M` | 在 Source / Markdown 模式间切换 |
| `Ctrl+S` | 立即落盘 |
| `Ctrl+Shift+C` | 复制默认值已填充的最终文本 |
| `↑` / `↓` | 移动列表选择（焦点留在搜索框） |
| `Enter` | 打开选中项，焦点进入画布 |
| `Esc` | 画布内退回搜索框；搜索框内清空搜索 |
| `Ctrl+C` / `Ctrl+A` | 仅 Markdown 模式：复制渲染选择 / 全选 |
| `Tab` | 在非输入控件间移动焦点（显示焦点环） |

## 数据存储

- 单一 SQLite 文件，`items` 等表按 `docs/data-model.md` 的 schema v1 建齐；正文以 Markdown 原文存储。
- 数据目录解析优先级：
  1. 环境变量 `PROMPTDECK_DATA_DIR`
  2. 可执行文件同级的 `portable.flag` → `.\data`
  3. 平台默认：Windows `%LOCALAPPDATA%\PromptDeck`、Linux `$XDG_DATA_HOME/promptdeck`
- 编辑历史（Version）在保存时静默累积（哈希去重 + ≥10s 限频），当前无历史 UI。
- 数据完全 Local-first，不联网、不上传；请勿手工编辑 SQLite。**卸载应用不会删除该数据目录。**

## Windows 原生构建

需 MSVC Build Tools + Windows SDK，以及 Windows 侧 rustup。WSL 下调用：

```bash
./scripts/build-windows.sh                 # release
./scripts/build-windows.sh --profile dev   # debug
```

产物：

```
D:\build\promptdeck\target\x86_64-pc-windows-msvc\release\promptdeck.exe
```

（脚本会 rsync 源码到 `D:\build\promptdeck`，再用 `cargo.exe --target x86_64-pc-windows-msvc` 构建。）

生成 Windows 安装包（需 Inno Setup 6，见 ADR-0009）：

```bash
./scripts/build-windows-installer.sh
```

产物：`D:\build\promptdeck\dist\PromptDeck-0.1.0-preview.1-windows-x64-setup.exe`（含 `.sha256`）。安装器使用项目图标（由 `ui/assets/app-icon.png` 生成多尺寸 `ui/assets/app.ico`）。

## 环境要求

- Rust stable（1.98.1 已验证），组件 rustfmt / clippy
- WSL：Slint 图形依赖（见 `report/environment-setup.md` 第 4 节）
- Windows：MSVC Build Tools + Windows SDK（见 `report/environment-setup.md`）

## 工程结构

- `crates/promptdeck-core` — 纯 Rust：领域模型、存储（rusqlite）、Markdown/Variables 解析，无 Slint 依赖
- `crates/promptdeck-app` — Slint UI 与装配层，二进制名 `promptdeck`

## 开发门槛

```bash
cargo fmt --check
cargo clippy --workspace --all-targets   # 0 警告
cargo test --workspace
```

## 当前限制

- Markdown 模式**只读**（ADR-0003 的已确认边界），富文本/块级混合编辑不在本阶段。
- Markdown 链接仅渲染样式，**不可点击打开浏览器**；代码块长行按词换行、无横向滚动。
- 快捷键 `Ctrl+K`（Quick Launcher）、变量填值表单、Version 历史 UI、删除/回收站 UI、导入/导出**尚未实现**。
- 列表行最多直接显示 3 个标签 chip，其余以「+N」表示。
- 搜索随输入即时查询、不参与选中；筛选后列表无可视选中时，`Enter` 仍打开画布当前条目（先按 `↓` 打开首个结果）。
- **Preview 状态**：未代码签名，Windows SmartScreen 可能提示「未知发布者」；暂无自动更新、Portable 包、商店发布。
- 采用静态 CRT（exe 无 VC++ 运行库依赖）；暂未确定 `LICENSE`。

## 文档

- `CONTEXT.md` — 领域术语表（唯一词汇来源）
- `docs/stage-01-handoff.md` — 当前交接报告与已知限制
- `docs/adr/` — 架构决策记录
- `docs/` — MVP 范围、模块边界、数据模型、切片
- `design/design.md` — 锁定的设计系统（Hallmark studied-DNA）
- `report/` — 环境基线与搭建报告
