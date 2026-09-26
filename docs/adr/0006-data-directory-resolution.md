# 数据目录解析：环境变量 → portable.flag → 平台默认

优先级：`PROMPTDECK_DATA_DIR` 环境变量 → 可执行文件同级存在 `portable.flag` 时用 `.\data` → Windows `%LOCALAPPDATA%\PromptDeck`、Linux `$XDG_DATA_HOME/promptdeck`（缺省 `~/.local/share/promptdeck`）。安装版绝不写安装目录；Portable 模式零注册表、零用户目录写入。

**Consequences**：首次启动在默认位置自动建库；「选择数据目录」向导与安装器集成留待发布阶段。当前实现见 `crates/promptdeck-core/src/paths.rs`。
