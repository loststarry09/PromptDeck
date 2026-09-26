# MVP 范围

状态：已确认（2026-09-26 grilling 三轮）。本文件定义 PromptDeck 第一个可用版本的边界；后续变更需显式修改本文件。

## 定位与成功标准

- 用户：作者本人优先（单一高级用户），同时按「陌生用户可直接使用」的标准设计；不做多用户、同步、协作。
- 核心任务：从可复用部件快速组装高质量 prompt，并送进目标应用。
- MVP 成功标准：不打开任何其他工具，就能完成「新建 → 编辑（Markdown）→ 自动保存 → 搜索找回 → 填充变量 → 复制到目标应用」的完整回路，且数据在重启后完好。

## In scope（MVP）

| 能力 | 说明 |
|---|---|
| Library | Prompt 的增删改查、收藏/置顶、Tags 过滤 |
| Source mode 编辑 | 单画布内纯 Markdown 编辑，防抖自动保存（~800ms） |
| Markdown mode | 同画布只读渲染（CommonMark 子集），左下角切换 |
| Variables | `{{name}}` / `{{name:默认值}}` 解析；使用时填值；`\{{` 转义 |
| 搜索 | FTS5（trigram，中文可用）+ 短查询 LIKE 兜底 |
| 复制 | Ctrl+Shift+C 复制变量已填充的最终文本 |
| Quick Launcher | 应用内命令面板（Ctrl+K），搜索 + 动作；无系统级热键 |
| Version 数据 | 哈希去重 + ≥10s 限频地累积修订；不提供历史 UI |
| 主题 | 跟随系统 + 手动切换；Light/Azure 与 Dark/Forest（design.md） |
| 数据目录 | `PROMPTDECK_DATA_DIR` → `portable.flag` → 平台默认（ADR-0006） |

## Out of scope（明确不做）

- Reusable Block 与 Composition Engine（v0.2；数据模型预留 `kind` 与引用位）
- Version 历史 UI（v0.3；数据从 MVP 起积累）
- Prompt Radar + Inbox（独立里程碑；隐私约束见 ADR-0004）
- 系统级全局热键（可选增强；必须用 OS 热键注册，禁用键盘钩子）
- 发布体系：Installer、Portable 打包、签名、静态 CRT、自动更新
- Folder 层级、i18n 运行时切换、值持久化、插件/脚本、云同步

## 里程碑顺序

1. **Slice 01 — Library 回路**（见 `docs/slices/01-library-loop.md`）：端到端编辑回路 + 正确外壳
2. **Slice 02 — Variables 体验 + Quick Launcher**：填值表单、默认值、复制动作、命令面板
3. **v0.2 — Reusable Blocks + Composition Engine**：块引用、组装、环检测
4. **v0.3 — Versions UI + 打磨**：历史、对比、恢复
5. **Radar + Inbox**：外部采集（剪贴板默认关、目录、手动）→ Inbox 整理入库

## 开放事项

- 许可证未定（未写 LICENSE）；发布前决定
- 块引用语法未定（v0.2 设计）
- 发布体系与静态 CRT、签名、防 SmartScreen 策略（发布阶段）
