# Slice 01 — Library 回路（tracer bullet）

状态：**已完成（2026-09-27）**。工单 01–07 全部交付并通过双端（WSLg + Windows 原生）验收；详见 `docs/stage-01-handoff.md`。目标是端到端跑通最小闭环，并把外壳一次做对（基于 design.md tokens），避免二次美化。

实现契约与工单见本地 tracker：`.scratch/slice-01-library-loop/spec.md` + `issues/01–07`（本地 markdown，未纳入 git）。

## 目标

打开应用即可完成：**新建 → Source 编辑（自动保存）→ 列表出现/搜索 → Ctrl+M 只读渲染 → Ctrl+Shift+C 复制**，重启后数据完好。

## 交付物

### core

- schema v1 迁移（`docs/data-model.md` 全部表 + FTS5 trigram）
- `model`：`Item`、`ItemKind`、`Tag`、`VariableDef`、`Revision`
- 仓储：items CRUD、tags 挂接、variables 派生写入、versions 去重写入、软删除
- `variables`：解析（`{{name}}` / `{{name:default}}` / `\{{`）与最终文本替换
- `search`：FTS5 查询 + <3 字符 `LIKE` 兜底
- `clock` / `id` 注入点（测试确定性）

### app

- `ui/theme.slint`：design.md tokens（Light/Azure + Dark/Forest，跟随系统 + 手动切换）
- 外壳：N3 side-rail + 224px 列表面板（搜索 + 列表 + Tags 过滤）+ 单画布（标题 + 编辑/渲染 + 左下模式切换器）
- 行为：Ctrl+N 新建、编辑防抖自动保存（~800ms）、Ctrl+M 切模式、Ctrl+Shift+C 复制、Ctrl+S flush、列表选中切换
- `strings` 集中中文文案

### 明确不做

变量填值表单、Quick Launcher 面板、历史 UI、设置页、删除/回收站 UI、Tags 编辑 UI、导入导出。

## 验收标准（Definition of Done）

1. `cargo fmt --check` 通过；`cargo clippy --workspace --all-targets` 0 警告；`cargo test --workspace` 全绿。
2. 从空库执行迁移后：items/tags/item_tags/variables/versions/settings/items_fts 均存在，二次迁移幂等。
3. 单测覆盖：
   - items CRUD 往返 + 软删除过滤
   - FTS 搜索：英文子串、中文子串（如「翻译」命中「请翻译成英文」）、<3 字符兜底
   - 变量解析：无变量 / 单变量 / 默认值 / 重复合并 / `\{{` 转义
   - 修订：首次保存落一条；同内容不重复；内容变化且 ≥10s 落新条（注入时钟）
4. 手工冒烟（WSLg 与 Windows 各一次）：
   - Ctrl+N 新建 → 输入含 `{{lang:英文}}` 的 Markdown → 列表出现
   - 重启后内容与列表保持
   - 搜索可命中中文子串；Ctrl+M 渲染效果正确（标题/粗体/列表/代码）
   - Ctrl+Shift+C 复制结果中 `{{lang:英文}}` 已被默认值替换；未填且无默认的变量保持占位
5. `./scripts/build-windows.sh` 成功，Windows GUI 运行无崩溃。

## 测试计划

| 层 | 文件 | 内容 |
|---|---|---|
| core 单测 | `storage/tests` 内联 | 迁移、CRUD、搜索、修订 |
| core 单测 | `variables` | 解析与替换表驱动 |
| core 单测 | `search` | 中英文与短查询 |
| 手工 | WSLg / Windows | 上述冒烟清单 |

## 依赖与风险

- Slint `StyledText` 对 CommonMark 子集渲染：标题/列表/代码/粗斜体可用；表格、脚注等不支持（渲染降级为纯文本，可接受）。
- trigram FTS 索引体积略大；单机库量级（几千条）无碍。
- OKLCH → sRGB 转换需要一次性校色；转换后按 design.md 的对比度要求抽查。
