# PromptDeck 环境基线报告（Windows + WSL，只读调查）

调查日期：2026-09-26。全程只读，未安装、升级、删除或修改任何系统、WSL、Rust、Visual Studio、PATH 或其他配置。

---

## 一、当前环境概况

| 项目 | 实测值 |
|---|---|
| Windows | Windows 11 家庭中文版，`10.0.26200.9457`（25H2），x64；主机 LAPTOP-ECABQR4Q |
| CPU | AMD Ryzen 9 7945HX，16 逻辑处理器 |
| WSL | WSL `2.7.11.0`，Ubuntu 24.04.4 LTS，内核 `6.18.33.2-microsoft-standard-WSL2`，WSLg 1.0.73.2 |
| WSL 配置 | `systemd=true`；interop `enabled=true` 但 **`appendWindowsPath=false`**；automount 默认（无 metadata） |
| `.wslconfig` | mirrored 网络、dnsTunneling、autoProxy；memory=8GB、processors=16、swap=4GB（D:\WSL\swap.vhdx） |
| 磁盘 | C: 201G（剩 59G）、D: 752G（剩 532G）、WSL ext4 1007G（剩 922G）；LongPathsEnabled=1 |
| 项目目录 | `/home/carlos/projects/promptdeck` 为空，未初始化 git |
| 关键工具 | winget 1.29.380、choco 2.7.0、Inno Setup 6.7.1（D:\Inno Setup 6）、Git for Windows 2.45.1（D:\Git）、VS Code 1.138.0（D:\Microsoft VS Code）+ Remote-WSL、Docker 29.7.2、Node 24.19.0（WSL） |
| Windows SDK | 10.0.22621.0 完整（Include/Lib/bin，含 rc.exe、signtool.exe） |
| 网络 | WSL 代理 `127.0.0.1:7897`；crates.io / static.rust-lang.org / win.rustup.rs / github 均 200 |
| 安全 | Defender 实时防护关闭（RTP=False），疑似第三方安全软件接管 |
| 互操作 | 绝对路径可运行 `cmd.exe`/`powershell.exe`/`wsl.exe`；`cmd.exe` 不支持 UNC 当前目录（会回退到 C:\Windows）；PowerShell 5.1 可处理 UNC |

**结论：Windows/WSL 底座良好，但开发/构建工具链几乎为零——Rust 与 MSVC 均未安装。**

## 二、已满足项

- Windows 11 25H2 x64 + WSL2（systemd、WSLg、162 个字体）稳定运行，具备 Slint 图形预览条件。
- 互操作可用（binfmt WSLInterop enabled），可从 WSL 调起 Windows 程序。
- Windows SDK 10.0.22621.0 完整（头文件/库/工具，含 `rc.exe`、`signtool.exe`），是 MSVC 链接所需的 SDK 部分。
- **Inno Setup 6.7.1 已安装**（D:\Inno Setup 6\ISCC.exe）→ 自定义安装路径/快捷方式/卸载开箱可用。
- WSL 具备 C 构建链：gcc 13.3、clang 18.1.3、cmake 3.28、make、ninja 1.11、pkg-config、python3.12、perl → `rusqlite bundled` 在 WSL 可编译。
- Windows 已有 VC 运行库（System32 下 `vcruntime140.dll`、`vcruntime140_1.dll`、`msvcp140.dll`，`ucrtbase.dll` 系统自带），本机测试无碍。
- winget/choco 可用于后续安装；D: 空间充足且 WSL 可写；Git 双端可用（WSL `core.autocrlf=input`）。
- 网络到 Rust/crates/GitHub 全部可达。

## 三、缺失或异常项

1. **Windows 无 Rust**：无 `rustup/cargo/rustc`，无 `%USERPROFILE%\.cargo`、`.rustup`，PATH 无 `.cargo\bin`。
2. **WSL 无 Rust**：无 `rustc/cargo/rustup/rust-analyzer`，无 `~/.cargo`；也无 `lld/mold`。apt 仅有 rustc 1.75（过旧，不建议）。
3. **无 MSVC C++ 工具链（硬阻塞）**：
   - `vswhere.exe` 不存在；`Program Files[\ (x86)]\Microsoft Visual Studio\{2022,18,Shared}` 全部为空；无 `cl.exe/link.exe`；注册表无 `SxS\VS7`。
   - choco 日志显示 2026-09-12 安装 `visualstudio2026buildtools` 时 **“Elevated State = Failed”（未提权）**；`visualstudio2026-workload-vctools` 遗留 `.chocolateyPending`；`C:\ProgramData\Microsoft\VisualStudio\Packages` 有 VC 14.50 缓存但 `_Instances` 为空 → 安装未完成。
4. 无 NSIS / WiX；仅有 .NET Runtime 7.0.20（无 SDK）→ WiX 4/5 暂不可用（不影响 Inno 方案）。
5. WSL 缺 Slint/Linux 构建所需 dev 包：`libfontconfig1-dev`、`libfreetype6-dev`、`libxkbcommon-dev`、`libgl1-mesa-dev` 及 X11 系列均未安装（仅有运行时 `.so`）。
6. VS Code 双端均无 `rust-analyzer`、无 Slint 扩展。
7. Developer Mode 未启用（仅影响 symlink/MSIX，非必需）；C 盘仅剩 59G，需规划安装位置。
8. 目录未初始化 git；`mise` 已装但未管理任何运行时。

## 四、风险与注意事项

- **UNC 构建不可取**：仓库在 ext4 时，从 WSL 调 Windows `cargo.exe` 会以 `\\wsl.localhost\...` 为工作目录——9p 性能差、`cmd.exe` 不支持 UNC cwd、部分 build script 可能失败、杀软扫描更慢。发布构建应使用 NTFS 上的副本。
- **crt-static 非默认**（RFC 1721：`x86_64-pc-windows-msvc` 默认动态链接 CRT）：不显式静态化则运行依赖 `VCRUNTIME140.dll`；若启用 Slint Skia 渲染器还会依赖 `MSVCP140.dll`。为“开箱即用/Portable”，应 `+crt-static`（`cc` 会同步以 `/MT` 编译 bundled SQLite）。
- **C 盘空间**：rustup/cargo 默认落 C 盘；建议 `CARGO_HOME`/`RUSTUP_HOME`/`target-dir` 放 D:。
- **choco 残留**：`.chocolateyPending` 与 VS 缓存可能干扰后续 Build Tools 安装，需先处理。
- **appendWindowsPath=false**：WSL 中必须用绝对路径调 Windows 工具（也避免了误执行 `.exe`，属可控代价）。
- **安全软件**：Defender RTP 关闭，实际 AV 未知；建议对 `target/`、`D:\build` 加排除。
- **换行符**：Windows Git system 级 `core.autocrlf=true`（global 为 input）；仍应加 `.gitattributes`（`* text=auto eol=lf`）统一。
- **Slint 细节**：Windows 需 `#![windows_subsystem="windows"]` 去控制台；debug 构建建议 `link-arg=/STACK:8000000` 防栈溢出；默认 femtovg(OpenGL) 失败时回退 software，勿启用 Skia 以保持轻量。
- **版本基线**（crates.io 实测）：slint `1.18.1`（2026-09-21）、rusqlite `0.40.2` / libsqlite3-sys `0.38.2`（bundled SQLite 3.53.2）；rusqlite MSRV = 发布时最新 stable → 必须用最新 stable 工具链。
- 安装版不能写 Program Files；**数据目录选择是必要设计**；未签名产物会触发 SmartScreen（后续可用 SDK signtool 签名）。

## 五、推荐的 WSL → Windows 原生构建链路

**推荐：WSL 作为唯一源码/Agent 工作区，Windows 仅做原生 MSVC 构建与打包（源码经 rsync 同步到 NTFS）。**

1. 源码常驻 `/home/carlos/projects/promptdeck`（ext4，git/Agent 最快）。
2. 发布构建同步到 `D:\build\promptdeck` 后用 Windows cargo（cwd 在 NTFS，规避 UNC）：
   ```bash
   rsync -a --delete --exclude target --exclude .git \
     /home/carlos/projects/promptdeck/ /mnt/d/build/promptdeck/
   cd /mnt/d/build/promptdeck && \
     /mnt/c/Users/Carlos/.cargo/bin/cargo.exe build --release --target x86_64-pc-windows-msvc
   ```
3. 快速迭代（可选）：WSL 内 `cargo check`/`clippy`（Linux 目标）做类型检查；如需 Linux 运行/预览 Slint，补装第三节所列 dev 包。rust-analyzer 走 VS Code Remote-WSL。
4. 项目内 `.cargo/config.toml` 建议：
   ```toml
   [build]
   target = "x86_64-pc-windows-msvc"
   target-dir = "D:\\build\\promptdeck-target"
   [target.x86_64-pc-windows-msvc]
   rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=/STACK:8000000"]
   ```
5. 备选（更简单但慢）：仓库放 `D:\Projects\promptdeck`，WSL 经 `/mnt/d` 访问，直接原生构建；代价是 git/Agent 的 9p I/O 变慢。
6. 验收：MSVC 装好后用 `vswhere` 确认实例；`dumpbin /dependents promptdeck.exe` 应只剩系统 DLL（KERNEL32/USER32/OPENGL32 等），无 VCRUNTIME140/MSVCP140。

## 六、推荐的 Installer / Portable 发布方案

- **产物形态**：单个 `PromptDeck.exe`（`+crt-static`、rusqlite `bundled`、femtovg + software 回退），无 WebView2/Node/Python/SQLite 依赖。
- **Installer（Inno Setup 6.7.1，已装）**：
  - 默认含“选择安装位置”向导页（`DisableDirPage=no`）；`ArchitecturesAllowed=x64compatible`；
  - `PrivilegesRequired=lowest` + `PrivilegesRequiredOverridesAllowed=dialog`（允许用户选择当前用户/全机安装）；
  - 开始菜单快捷方式 + 可选桌面快捷方式；标准卸载（unins000.exe）；升级用固定 `AppId`；
  - 安装目录只放程序与可选资源，**不写业务数据**。
- **Portable**：zip 解压即用；目录含 `PromptDeck.exe`、`data\`、`portable.flag`、README；全部路径基于 `current_exe()`，不写注册表、不写用户目录。
- **数据目录解析优先级（建议）**：`PROMPTDECK_DATA_DIR` 环境变量/CLI → `portable.flag`（用 `.\data`）→ 配置文件 → 首次启动对话框选择。安装版小配置放 `%APPDATA%\PromptDeck\config.json`，默认数据目录 `%LOCALAPPDATA%\PromptDeck`（可改到任意盘）；SQLite 建议 WAL + `foreign_keys=ON`，DB/资源/日志统一落在所选数据目录。

## 七、正式初始化前的最小准备清单

1. **[必须，管理员]** 安装 MSVC：VS 2022/2026 Build Tools + “使用 C++ 的桌面开发”（VC++ x64 + Windows SDK）。可用 `winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`；先清理 choco 遗留（`visualstudio2026-workload-vctools` pending 状态）。验收 `vswhere` + `link.exe`。
2. **[必须]** Windows 安装 rustup + stable（默认 `x86_64-pc-windows-msvc`），组件 `rustfmt`/`clippy`/`rust-analyzer`；`CARGO_HOME`/`RUSTUP_HOME` 建议放 D:。
3. **[推荐]** WSL 安装 rustup + stable（供 check/clippy/rust-analyzer）；如需 Linux 运行 Slint，补装 `libfontconfig1-dev libfreetype6-dev libxkbcommon-dev libx11-dev libx11-xcb-dev libxcursor-dev libxkbcommon-x11-dev`。
4. **[推荐]** VS Code：WSL 侧装 `rust-lang.rust-analyzer` 与 `slint.slint` 扩展。
5. **[必须]** 确定仓库/构建布局（推荐 ext4 仓库 + `D:\build` 同步；备选仓库放 D:）。
6. **[推荐]** 脚手架即写入：`.gitattributes`、`.cargo/config.toml`（msvc + target-dir + crt-static + /STACK）、`rust-toolchain.toml`（pin stable）、`.gitignore`（`/target`、数据目录）。
7. **[推荐]** 对实际 AV 添加 `target/`、`D:\build` 排除；**[可选]** 启用 Developer Mode、准备代码签名证书。
8. **[验收冒烟]** 空项目 `cargo build --release` → `dumpbin /dependents` 无 VCRUNTIME；再以最小 slint + rusqlite(bundled) 项目验证编译与链接，然后才正式初始化 PromptDeck。
