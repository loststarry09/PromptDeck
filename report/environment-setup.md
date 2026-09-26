# PromptDeck 开发环境搭建结果（Windows + WSL）

执行日期：2026-09-26。基于 `environment-baseline.md`，按"精简版"范围执行（不含 Installer/Portable/签名/静态 CRT/发布链/性能优化）。

---

## 一、最终结果速览

| 项目 | 结果 |
|---|---|
| Windows Rust | rustup 1.29.1 + stable 1.98.1（`x86_64-pc-windows-msvc`），路径 `D:\Rust\cargo` / `D:\Rust\rustup` |
| Windows 组件 | rustfmt 1.9.0、clippy 0.1.98、rust-analyzer 1.98.1 |
| MSVC | VS 2022 BuildTools 17.14.37710.0，MSVC 工具集 14.44.35207（cl.exe/link.exe Hostx64\x64） |
| Windows SDK | 10.0.22621.0（原有，um/ucrt x64 库齐全；**补写了缺失的注册表键**） |
| WSL Rust | rustup 1.29.1 + stable 1.98.1（`x86_64-unknown-linux-gnu`），默认 `~/.cargo` / `~/.rustup`（ext4，VHDX 在 `D:\WSL\Ubuntu-24.04`） |
| VS Code Remote-WSL | `rust-lang.rust-analyzer` v0.3.3057、`slint.slint` v1.18.1 |
| WSL 图形/构建库 | fontconfig 2.15.0、freetype 26.1.20、xkbcommon、xkbcommon-x11、xcursor、x11-xcb、libgl1-mesa-dev |
| Smoke test | `.scratch/env-smoke`（Slint 1.18.1 + rusqlite 0.40 bundled），WSL 与 Windows 双端构建/测试/GUI 运行全部通过 |
| 业务代码 | 未初始化 git、未编写 PromptDeck 业务代码 |

## 二、实际安装与路径

### 1. Windows Rust
- 安装方式：`rustup-init.exe -y --default-toolchain stable -c rustfmt -c clippy -c rust-analyzer`（12.7MB 引导器存于 `D:\build\tools\`）
- `CARGO_HOME=D:\Rust\cargo`、`RUSTUP_HOME=D:\Rust\rustup`（**用户级**环境变量，未动系统级）
- 用户 PATH 已由 rustup 添加 `D:\Rust\cargo\bin`（在写此报告时为用户 PATH 首位）
- 验证：`cargo 1.98.1` / `rustc 1.98.1` / `rustfmt 1.9.0-stable` / `clippy 0.1.98` / `rust-analyzer 1.98.1`；`rustup show` host = `x86_64-pc-windows-msvc`

### 2. MSVC C++ 构建链
- winget `Microsoft.VisualStudio.2022.BuildTools` 17.14.41（静默安装，组件 `Microsoft.VisualStudio.Workload.VCTools` + `Microsoft.VisualStudio.Component.VC.Tools.x86.x64`，复用现有 SDK）
- 安装路径：`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`（占 2.9GB）
- 验证：`vswhere.exe` 报实例 17.14.37710.0；`VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\{cl,link}.exe` 存在；SDK `Lib\10.0.22621.0\{um,ucrt}\x64` 齐全

### 3. WSL Rust
- `sh.rustup.rs` 官方脚本安装，走本机代理 `127.0.0.1:7897`
- `~/.cargo/bin` 已写入 `~/.profile` 与 `~/.bashrc`
- 验证：`cargo/rustc 1.98.1`，工具链 `stable-x86_64-unknown-linux-gnu`，组件含 rust-analyzer

### 4. WSL 系统依赖（sudo，一次性脚本）
`.scratch/env-smoke/install-wsl-slint-deps.sh` 安装：`libfontconfig1-dev libfreetype-dev libxkbcommon-dev libxkbcommon-x11-dev libxcursor-dev libx11-xcb-dev libgl1-mesa-dev`

### 5. VS Code 扩展（WSL 侧）
`rust-lang.rust-analyzer` 0.3.3057、`slint.slint` 1.18.1（Windows 侧不装，走 Remote-WSL）

## 三、执行中修复的问题

### A. VS/choco 残留清理（提权）
- 删除 4 个未完成的 choco 包：`visualstudio2026buildtools`、`visualstudio2026-workload-vctools`、`visualstudio-installer`、`chocolatey-visualstudio.extension`
- 删除 4 个空壳目录（`Program Files[\ (x86)]\Microsoft Visual Studio\{2022,18,Shared}`）与 52MB 部分包缓存

### B. **关键根因：Windows SDK 注册表注册不完整**（导致 `LNK1181: 无法打开 kernel32.lib`）
- 现象：MSVC 已装好，但 rustc 不向 link.exe 传 SDK 库路径；强制 `LIBPATH` 可链接成功
- 定位：rustc 1.98 使用 `find-msvc-tools` 0.1.5 探测 SDK；其回退路径读取 **32 位视图**的
  `HKLM\SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0\InstallationFolder`，本机该键不存在（SDK 安装未写全注册表）
- 修复：补写该键（64 位视图同步补写）：
  ```
  HKLM\SOFTWARE\WOW6432Node\Microsoft\Microsoft SDKs\Windows\v10.0  InstallationFolder = C:\Program Files (x86)\Windows Kits\10\
  HKLM\SOFTWARE\Microsoft\Microsoft SDKs\Windows\v10.0            InstallationFolder = C:\Program Files (x86)\Windows Kits\10\
  ```
- 修复后 `rustc` 无需任何环境变量即可正常链接；`cc`（bundled SQLite）同样受益

### C. WSL→Windows 环境变量透传
- 实测普通变量默认**不传**给 Windows 进程，需 `WSLENV` 列表；构建脚本用 `WSLENV=...:CARGO_HOME:RUSTUP_HOME` 传递

### D. Slint 1.18 语法
- `VerticalBox` 已移除 → `VerticalLayout`；枚举值需限定（`LayoutAlignment.center`、`TextHorizontalAlignment.center`）

## 四、Smoke test 与构建命令

位置：`/home/carlos/projects/promptdeck/.scratch/env-smoke`（scratch，非业务代码）

```
Cargo.toml        slint = "1"，rusqlite = { features = ["bundled"] }，build-dep slint-build
build.rs          slint_build::compile("ui/app.slint")
ui/app.slint      最小窗口，显示 SQLite 版本
src/main.rs       rusqlite 内存库 + #[test] 建表/插入/查询
build-windows.sh  同步到 D:\build\env-smoke 并原生构建（动态探测 cargo.exe，显式 --target）
install-wsl-slint-deps.sh  一次性 sudo 系统依赖
```

**WSL 日常开发（在 `.scratch/env-smoke` 下）：**
```bash
~/.cargo/bin/cargo check
~/.cargo/bin/cargo test          # 已验证 bundled SQLite 往返
~/.cargo/bin/cargo clippy --all-targets
~/.cargo/bin/cargo build         # Linux GUI 可经 WSLg 运行
```

**Windows 原生 `.exe`：**
```bash
./build-windows.sh
# 等价于：
# rsync -a --delete --exclude target <src>/ /mnt/d/build/env-smoke/
# cd /mnt/d/build/env-smoke && cargo.exe build --release --target x86_64-pc-windows-msvc
# 产物：D:\build\env-smoke\target\x86_64-pc-windows-msvc\release\env-smoke.exe
```
- 不设全局默认 target；始终显式 `--target x86_64-pc-windows-msvc`（避免影响 WSL 构建）
- 构建副本位于 NTFS（`D:\build`），规避 UNC cwd；`CARGO_HOME`/`RUSTUP_HOME`/源码仓库均不在 C 盘

**验收记录：**

| 验收项 | 结果 |
|---|---|
| WSL `cargo check` / `test` / `clippy` / `fmt --check` | 全部通过，0 警告 |
| WSL GUI（WSLg）运行 5s | 正常（timeout 结束，无崩溃） |
| Windows `cargo test --target msvc` | 1 passed（bundled SQLite） |
| Windows `cargo build --release --target msvc` | 成功，PE32+ 13.6MB |
| Windows GUI 运行 10s | 正常（timeout 结束，无崩溃） |

## 五、仍存在的问题 / 已知限制

1. **动态 CRT**：Windows 产物按当前阶段保持默认动态链接，依赖系统 `VCRUNTIME140.dll`（本机已存在）。静态 CRT（Portable 前提）推迟到 MVP 后。
2. **未做**：Installer、Portable 模式、代码签名、自定义发布链、性能调优——均按简化范围推迟。
3. **C 盘余量**：BuildTools 装在 C 盘，占用约 2.9GB；C: 剩约 56GB。Rust 工具链与构建缓存均在 D 盘。
4. **构建目录约定**：Windows 构建必须用 NTFS 副本（`D:\build\...`），不要从 `/mnt/c` 或 UNC 路径直接构建；`build-windows.sh` 已封装。
5. **SDK 注册表修复是本机特定补丁**：若未来 SDK 升级/重装，正常安装器应自行写全该键；否则需重新补写。
6. **AV/Defender**：实时防护关闭、实际 AV 未知。可选：对 `D:\build` 与 WSL 项目 `target/` 加排除以提速。
7. **未初始化 git**：仓库尚无 `.gitattributes`/`.gitignore`/`rust-toolchain.toml`；正式初始化 PromptDeck 时应一并建立（本项目 smoke test 未引入这些文件）。
8. `rust-analyzer` 已装双端，但 VS Code 工作区设置（如 `rust-analyzer.cargo.targetDir` 等）未配置——首次打开项目时按需再调。

## 六、原始日志与脚本位置

- 日志：`D:\build\logs\{cleanup-vs-leftovers,install-vs-buildtools,fix-sdk-registry}.log`
- 一次性脚本：`D:\build\tools\{cleanup-vs-leftovers,install-vs-buildtools,fix-sdk-registry}.ps1`
- 引导器：`D:\build\tools\rustup-init.exe`
