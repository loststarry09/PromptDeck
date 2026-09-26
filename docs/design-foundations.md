# 设计基础（Design Foundations）

状态：基线（2026-09-26）。设计系统真源为 `design/design.md`（Hallmark studied-DNA，锁定；如需变更须显式修订该文件）。本文件只记录 PromptDeck 如何把该 DNA 落到 Slint。

## 来源与边界

- 来源：`design/design.md`（BlockHelm Workbench DNA：N3 side-rail、Workbench macrostructure、Light/Azure 与 Dark/Forest）。
- 只取结构与抽象视觉原则（布局、层级、密度、动效纪律），不复制任何品牌、图标、像素资产或文案。
- 图标不来自 design.md：UI 图标取自 iconfont「阿里巴巴国际站官方图标库」（cid=19238，公开库·原创·免费使用）；应用窗口图标为用户设计稿。
- Hallmark 纪律在本项目生效：锁定 tokens、交互态完整、禁止重绘窗口装饰、诚实文案（空状态不编造数据）、标题一律正体（禁斜体标题）。

## 结构映射

| design.md | PromptDeck |
|---|---|
| Workbench macrostructure | 单窗口：side rail + context panel + single canvas |
| Primary rail · N3 | 62px 收起 / 176px 展开（底部按钮切换、状态持久化，不随 hover）；Library、Inbox（占位）、Settings |
| Context panel · 224px | 搜索框 + Prompt 列表 + Tags 过滤 |
| Work title/search layer | 画布首行：标题 + 收藏/置顶 + 复制动作 |
| Content max 800px | 编辑区最大宽度 800px，居中 |

## Tokens 落地规则

- `design/design.md` 的 OKLCH 值为规范值；实现时转换为 sRGB 写入 `crates/promptdeck-app/ui/theme.slint`（Slint 颜色字面量），作为运行期真源。
- 禁止在 `theme.slint` 之外出现硬编码颜色/字体；新值先入 tokens 再引用。
- 主题：Light/Azure 默认（跟随系统），可切换 Dark/Forest；切换只改颜色与高度层级，不动几何与排版。
- 标题/正文字体：`Microsoft YaHei UI` → `Segoe UI Variable` → system-ui；等宽 `Cascadia Mono` → Consolas；WSL 下由 fontconfig 回退。
- 图标：`ui/icons/*.svg`（单色 path）经 Slint `Image.colorize` 按 tokens 着色，统一由 `ui/icons.slint` 的 `AppIcon` 渲染；应用窗口图标 `ui/assets/app-icon.png`（1254px 原稿派生 512px）经 `Window.icon` 设置。

## 组件与状态纪律

- 列表行内容 54px、行距 62px（分隔线上下各留 4px）；名称（强）+ 元数据（弱）+ 收藏标记；1px `rule` 分隔线弱于 hover 态。
- 按钮、列表行、分段切换器实现 default / hover / active / disabled 状态。文本输入（标题、搜索）实现 default / focus / disabled：Slint 的 `TextInput` 不暴露 hover，覆盖层又会破坏点击定位光标，故输入组件不设 hover 态（与 Slint 内置 `LineEdit` 及既有标题输入一致）；无禁用场景前不预留 disabled 样式。loading / error / success 按组件需要。
- 焦点指示：2026-09-26 产品决策移除焦点环；按钮、列表等非输入元素不提供聚焦高亮；文本输入保留底部 accent 下划线作为聚焦提示（标题、搜索）。不依赖颜色单独表达选中/错误/成功。
- 单手操作目标 ≥44×44px（紧凑窗口下亦然）。
- 主 CTA 每区域一个；accent 覆盖 5–15%，不滥用。

## 动效

- 页面进入：22px 位移 + 淡入，240ms ease-out；hover 160ms 进 / 220ms 出；rail 62↔176px，360ms ease-in-out。
- 禁止回弹/过冲；交互图像缩放上限 1.04。
- `prefers-reduced-motion` 等价设置时：最多 150ms 不透明度过渡，去掉位移与缩放。

## 待办（首个切片内完成）

- [ ] OKLCH → sRGB 转换并生成 `ui/theme.slint`（Light/Azure + Dark/Forest）
- [ ] 外壳骨架：N3 rail、224px 列表面板、单画布、左下角模式切换器
- [ ] 模式切换器、列表行、搜索框的状态清单落地（7 态）
- [ ] 空状态：诚实的引导文案（无编造数据）
