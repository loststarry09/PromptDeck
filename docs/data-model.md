# 数据模型

状态：基线（2026-09-26）。SQLite 为唯一真源（ADR-0002）。标记为「未来」的表不进入首个切片迁移。

## 原则

- ID：UUID v7（TEXT，小写规范形式），按时间有序，便于列表与分页。
- 时间：UTC Unix 毫秒（INTEGER）。
- 内容：`body_md` 一律为 Markdown 原文；渲染格式不落库。
- 软删除：`items.deleted_at` 从第一天就有（回收站 UI 后置）。
- 迁移：`storage::MIGRATIONS` 的索引即 schema 版本（`PRAGMA user_version`）。**公开发布前允许原地修改迁移 1**；首次公开发布后冻结，只允许追加。开发期 schema 变更后删除本地库重建即可。

## Schema v1

```sql
CREATE TABLE items (
    id         TEXT PRIMARY KEY,
    kind       TEXT NOT NULL CHECK (kind IN ('prompt', 'block')),
    title      TEXT NOT NULL,
    body_md    TEXT NOT NULL,
    pinned     INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER
) STRICT;

CREATE INDEX idx_items_kind_updated ON items (kind, deleted_at, updated_at DESC);

CREATE TABLE tags (
    name TEXT PRIMARY KEY COLLATE NOCASE
) STRICT;

CREATE TABLE item_tags (
    item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    tag     TEXT NOT NULL REFERENCES tags(name) ON DELETE CASCADE ON UPDATE CASCADE,
    PRIMARY KEY (item_id, tag)
) STRICT, WITHOUT ROWID;

CREATE INDEX idx_item_tags_tag ON item_tags (tag);

CREATE TABLE variables (
    item_id       TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    name          TEXT NOT NULL,
    default_value TEXT,
    PRIMARY KEY (item_id, name)
) STRICT, WITHOUT ROWID;

CREATE TABLE versions (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id      TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    title        TEXT NOT NULL,
    body_md      TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at   INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_versions_item ON versions (item_id, created_at DESC);

CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

CREATE VIRTUAL TABLE items_fts USING fts5(
    title,
    body_md,
    item_id UNINDEXED,
    tokenize = 'trigram'
);
```

设计要点：

- **统一 items + kind**：Prompt 与未来 Reusable Block 共用一张表，标签/版本/搜索无需迁移即可覆盖 block。
- **FTS 用 trigram**：`unicode61` 对中文不分词，trigram 支持中英文子串匹配；不足 3 字符的查询由仓储层回退 `LIKE`。FTS 与 items 在同一事务内由仓储维护（不用触发器，便于测试与推理）；打开时若索引行数与在用条目数不一致（FTS 维护上线前的既有库，或部分写入的索引），仓储按 rowid 重建。
- **variables 为派生数据**：保存时解析正文写入；同名变量只存一次（首次出现的默认值生效）。
- **versions 为内容快照**：`title + body_md`；`content_hash` 用于去重。

## Variables 规则（ADR-0008）

- 语法：`{{name}}`，默认值可写 `{{name:默认值}}`；`name` 为不含空白、`{`、`}`、`:` 的 1+ 字符。
- 转义：`\{{` 渲染为字面 `{{`。其余反斜杠原样保留。
- 重复：同名多次出现合并为一条定义，默认值取首次出现。
- 代码块中的 `{{}}` 同样解析（MVP 接受；可转义规避）。
- MVP 不持久化用户填过的值，不引入类型/描述/作用域。

## 修订策略（ADR-0007）

- 自动保存 flush 时计算 `title + body_md` 的哈希；与最新修订哈希不同且距上一修订 ≥10s 时写入新修订。
- 新建条目的首次保存立即写入第一条修订。
- 哈希相同不重复写入；`≥10s` 窗口由可注入时钟控制，保证可测。

## 未来实体（仅为设计记录，不进 v1）

- **块引用 / Composition（v0.2）**：引用语法未定；解析产物落一张 `item_refs(source_item_id, target_item_id, position)` 风格的表，环检测在解析层完成。
- **Inbox（v0.4）**：`inbox_items(id, source, body_md, captured_at, state)`；接受后转为 item，丢弃走删除。
- **变量值历史（post-MVP）**：按变量名记忆常用值；需要用户可见的开关。
