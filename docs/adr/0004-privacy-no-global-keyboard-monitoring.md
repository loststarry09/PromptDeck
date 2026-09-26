# 隐私姿态：无全局键盘监听；Radar 本地处理、默认关、不出网

任何功能不得安装键盘钩子或做全局输入监听。Prompt Radar 的采集只来自用户显式开启的通道（剪贴板默认关闭、手动粘贴、指定目录），分析全部在本地完成，不发起网络请求。应用内 Quick Launcher 用普通快捷键。

**Consequences**：未来若提供系统级热键，必须使用操作系统热键注册（Windows `RegisterHotKey` / `WM_HOTKEY`），而非低层钩子；`win-hotkeys` 一类基于钩子的库被排除。Radar 的相似度先用词法（FTS/trigram），本地模型后议且必须离线。
