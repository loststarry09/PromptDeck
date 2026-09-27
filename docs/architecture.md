# 架构

状态：基线（2026-09-26）。模块边界按首个切片需要定义，后续按 ADR 演进。

## Crate 与职责

```
crates/promptdeck-core   # 纯 Rust，无 Slint；可被未来 CLI / 系统集成复用
crates/promptdeck-app    # Slint UI 与装配层；二进制名 promptdeck
```

依赖方向严格单向：`promptdeck-app → promptdeck-core → rusqlite`。core 不得引用 Slint，app 不得直接写 SQL。

## 模块边界（core）

| 模块 | 职责 | 首个切片 |
|---|---|---|
| `paths` | 数据目录解析（ADR-0006） | 已有 |
| `storage` | 连接、PRAGMA、迁移、仓储（items/tags/variables/versions/settings） | 扩展 |
| `model` | 领域结构：`Item`、`ItemKind`、`Tag`、`VariableDef`、`Revision` | 新增 |
| `variables` | 从 Markdown 解析变量定义；最终文本替换 | 新增 |
| `markdown` | Markdown 块级解析（标题/段落/列表/引用/代码/分隔线），无 Slint | 新增 |
| `search` | FTS5 查询与短查询兜底 | 新增 |
| `clock` / `id` | 时间与 ID 提供者（可注入，保证测试确定性） | 新增 |
| `error` | 领域错误（thiserror） | 新增 |

未来：`composition`（v0.2 块组装）、`capture`（Radar 采集适配器，v0.4）。

## 模块边界（app）

| 模块 | 职责 |
|---|---|
| `main` | 装配根：解析数据目录、打开并迁移数据库、构造控制器、运行窗口 |
| `controller` | 把 core 操作映射为 Slint 属性/回调；持有列表模型、选中状态与按条目的画布模式 |
| `markdown` | 把 core 的 Markdown 块映射为 Slint 块模型与 atom 字符区间；行内格式以 atom 属性表达（无 `StyledText`） |
| `selection` | Markdown 只读选择模型：atom 几何收集、命中测试、选择规范化与文本提取（纯函数） |
| `strings` | 中文文案集中管理，为 i18n 留位 |
| `ui/*.slint` | `app.slint`（窗口）、`theme.slint`（tokens）、`shell/library/canvas/markdown` |

## 数据流与线程模型

- MVP 单线程：Slint 事件循环内同步调用 core（本地 SQLite + WAL，微秒级；`busy_timeout=5000`）。
- 防抖自动保存由 UI 定时器触发 flush；切换条目、退出、`Ctrl+S` 强制 flush。
- 未来重活（Radar 分析、导入导出、本地相似度）进后台线程，经 `slint::invoke_from_event_loop` 回投 UI。

## 错误处理

- core 返回类型化错误（`thiserror`），绝不 panic 于用户数据；装不下/解析失败按可恢复错误上抛。
- app 边界统一映射为状态提示（状态行/横幅），不弹阻塞对话框（MVP）。

## UI 结构（首个切片）

```
Window
└─ HorizontalLayout
   ├─ Side rail（N3：Library / Inbox(占位) / Settings）
   ├─ Context panel（224px：搜索框 + Prompt 列表 + Tags 过滤）
   └─ Single canvas（标题 + Source 编辑 / Markdown 只读渲染 + 左下角模式切换）
```

## 测试策略

- core：内存 SQLite 单测（迁移、CRUD、搜索、修订去重）；变量解析表驱动测试；中文搜索用例。
- app：双端构建验证 + WSLg / Windows GUI 冒烟；MVP 不引入 UI 自动化。
- 每次提交门槛：`cargo fmt --check`、`cargo clippy --workspace --all-targets`（0 警告）、`cargo test --workspace`。

## 构建与运行

见 `README.md` 与 ADR-0005。WSL 做快速回路（Linux 目标），Windows 用 `scripts/build-windows.sh`（NTFS 镜像 + MSVC）。
