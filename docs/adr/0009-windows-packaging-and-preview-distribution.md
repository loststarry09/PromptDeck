# Windows 打包与 Preview 分发：Inno Setup + 静态 CRT + GitHub Releases

面向 Windows x64 普通用户分发 Preview 版本：用 **Inno Setup 6** 生成单文件 `.exe` 安装包，**每用户安装**（`PrivilegesRequired=lowest`，无 UAC，允许用户选择安装目录），程序图标与版本资源在构建时由 `embed-resource` 经 `crates/promptdeck-app/app.rc` 嵌入 exe（图标源为既有 `ui/assets/app-icon.png`，多尺寸 `.ico` 由 `ui/assets/app.ico` 提供）；采用**静态 CRT**（`.cargo/config.toml` 中对 `x86_64-pc-windows-msvc` 加 `+crt-static`），使 exe 不依赖 `VCRUNTIME140.dll`。版本号单一来源于工作区 `Cargo.toml`（`version.workspace`），安装器与 exe 版本资源保持一致。Preview 通过 GitHub **Pre-release** 分发（tag `vX.Y.Z-preview.N`），随附 SHA256。

**Considered Options**：WiX v4 / cargo-wix（MSI，企业友好但配置重）；NSIS（脚本老化、诊断差）；Portable zip（无需安装，但不满足「开始菜单启动 + 可卸载」目标）；动态 CRT + 内附 VC++ Redist（多一步依赖安装，Preview 阶段不划算）。

**Consequences**：不做代码签名——Windows SmartScreen 可能提示「未知发布者」，属 Preview 已知限制，不尝试绕过；不做自动更新、Portable、商店发布、签名体系。用户数据在平台默认目录（Windows `%LOCALAPPDATA%\PromptDeck`，ADR-0006），不在 `{app}` 内，安装器/卸载器**不写入也不删除**，故卸载保留用户数据（已实测：卸载前后 DB 哈希一致）。打包仅存在于 Windows 构建脚本（`scripts/build-windows-installer.sh` + `packaging/windows/promptdeck.iss`），WSL 侧只做 fmt/clippy/test。
