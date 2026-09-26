# 构建链路：WSL 源码，NTFS 镜像 + Windows MSVC 原生构建

源码常驻 WSL ext4（git 与 Agent 最快）；Windows 构建经 rsync 镜像到 NTFS（`D:\build\promptdeck`）后用 Windows `cargo.exe` + MSVC 构建，始终显式 `--target x86_64-pc-windows-msvc`，不做跨编译。WSL 侧以 Linux 目标跑 `check`/`clippy`/`test` 快速回路。

**Considered Options**：仓库放 NTFS 直接构建——9p I/O 慢、`cmd.exe` 不支持 UNC cwd、build script 易踩坑。

**Consequences**：Windows 构建统一走 `scripts/build-windows.sh`；NTFS 构建副本不是源码真源，切勿在副本上编辑。不要设置全局默认 target，避免污染 WSL 构建。
