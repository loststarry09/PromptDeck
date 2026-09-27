use std::path::Path;
use std::str::FromStr;

use rusqlite::{Connection, OptionalExtension, named_params, params};

use crate::clock::{Clock, SystemClock};
use crate::error::{Error, Result};
use crate::id::{IdSource, UuidV7Ids};
use crate::model::{
    Item, ItemKind, ItemSummary, Revision, Tag, VariableDef, content_hash, derive_title,
};
use crate::search::{self, QueryPlan};
use crate::variables;

pub const REVISION_WINDOW_MS: i64 = 10_000;
pub const SETTING_THEME: &str = "theme";
pub const SETTING_SELECTED_PROMPT: &str = "library.selected_prompt";
pub const SETTING_RAIL_EXPANDED: &str = "rail.expanded";

const ITEM_COLUMNS: &str = "id, kind, title, body_md, pinned, created_at, updated_at, deleted_at";
const SUMMARY_COLUMNS: &str = "i.id, i.kind, i.title, i.pinned, i.created_at, i.updated_at";
const SUMMARY_ORDER: &str =
    "ORDER BY i.pinned DESC, i.updated_at DESC, i.created_at DESC, i.id DESC";
/// Narrows a prompt query to Prompts carrying `:tag`, case-insensitively. A
/// `NULL` `:tag` disables the filter, so the same SQL serves unfiltered search.
const TAG_FILTER: &str = "AND (:tag IS NULL OR EXISTS (
         SELECT 1 FROM item_tags it
         WHERE it.item_id = i.id AND it.tag COLLATE NOCASE = :tag COLLATE NOCASE))";

pub struct Library {
    conn: Connection,
    clock: Box<dyn Clock>,
    ids: Box<dyn IdSource>,
}

impl Library {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(path, SystemClock, UuidV7Ids)
    }

    pub fn open_with(
        path: impl AsRef<Path>,
        clock: impl Clock + 'static,
        ids: impl IdSource + 'static,
    ) -> Result<Self> {
        let conn = crate::storage::open(path.as_ref())?;
        Self::from_connection(conn, clock, ids)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with(SystemClock, UuidV7Ids)
    }

    pub fn open_in_memory_with(
        clock: impl Clock + 'static,
        ids: impl IdSource + 'static,
    ) -> Result<Self> {
        let conn = crate::storage::open_in_memory()?;
        Self::from_connection(conn, clock, ids)
    }

    fn from_connection(
        conn: Connection,
        clock: impl Clock + 'static,
        ids: impl IdSource + 'static,
    ) -> Result<Self> {
        crate::storage::migrate(&conn)?;
        ensure_fts_index(&conn)?;
        backfill_variables(&conn)?;
        Ok(Self {
            conn,
            clock: Box::new(clock),
            ids: Box::new(ids),
        })
    }

    pub fn now_ms(&self) -> i64 {
        self.clock.now_ms()
    }

    pub fn create_prompt(&self) -> Result<Item> {
        let now = self.clock.now_ms();
        let item = Item {
            id: self.ids.next_id(),
            kind: ItemKind::Prompt,
            title: String::new(),
            body_md: String::new(),
            pinned: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };

        let transaction = self.conn.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO items (id, kind, title, body_md, pinned, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                item.id,
                item.kind.as_str(),
                item.title,
                item.body_md,
                item.pinned,
                item.created_at,
                item.updated_at,
            ],
        )?;
        sync_fts(&transaction, &item.id, &item.title, &item.body_md)?;
        transaction.commit()?;

        Ok(item)
    }

    pub fn save(&self, id: &str, title: &str, body_md: &str) -> Result<Item> {
        let transaction = self.conn.unchecked_transaction()?;
        let current =
            load_item(&transaction, id)?.ok_or_else(|| Error::ItemNotFound(id.to_string()))?;

        let title = if title.trim().is_empty() {
            derive_title(body_md)
        } else {
            title.to_string()
        };
        let hash = content_hash(&title, body_md);

        if hash == content_hash(&current.title, &current.body_md) {
            return Ok(current);
        }

        let now = self.clock.now_ms();
        transaction.execute(
            "UPDATE items SET title = ?1, body_md = ?2, updated_at = ?3 WHERE id = ?4",
            params![title, body_md, now, id],
        )?;
        sync_fts(&transaction, id, &title, body_md)?;
        sync_variables(&transaction, id, body_md)?;

        let latest: Option<(String, i64)> = transaction
            .query_row(
                "SELECT content_hash, created_at FROM versions
                 WHERE item_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;

        let should_write_revision = match latest {
            None => true,
            Some((latest_hash, latest_at)) => {
                latest_hash != hash && now - latest_at >= REVISION_WINDOW_MS
            }
        };

        if should_write_revision {
            transaction.execute(
                "INSERT INTO versions (item_id, title, body_md, content_hash, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, title, body_md, hash, now],
            )?;
        }

        transaction.commit()?;
        self.load(id)?
            .ok_or_else(|| Error::ItemNotFound(id.to_string()))
    }

    pub fn load(&self, id: &str) -> Result<Option<Item>> {
        load_item(&self.conn, id)
    }

    /// Sets an item's pinned flag. Pinning is a separate axis from recency:
    /// `updated_at` is deliberately left untouched, so unpinning returns the
    /// Prompt to its original place in the update order.
    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<Item> {
        self.conn.execute(
            "UPDATE items SET pinned = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![pinned, id],
        )?;
        self.load(id)?
            .ok_or_else(|| Error::ItemNotFound(id.to_string()))
    }

    /// The derived Variable definitions for an item, ordered by name.
    pub fn variables(&self, id: &str) -> Result<Vec<VariableDef>> {
        let mut statement = self.conn.prepare(
            "SELECT name, default_value FROM variables WHERE item_id = ?1 ORDER BY name",
        )?;
        let rows = statement.query_map([id], |row| {
            Ok(VariableDef {
                name: row.get(0)?,
                default_value: row.get(1)?,
            })
        })?;

        let mut variables = Vec::new();
        for row in rows {
            variables.push(row?);
        }
        Ok(variables)
    }

    /// The Tag names attached to an item, ordered case-insensitively.
    pub fn tags(&self, id: &str) -> Result<Vec<Tag>> {
        tags_for(&self.conn, id)
    }

    /// Attaches a Tag to a Prompt and returns its canonical stored name. Names
    /// are trimmed and unique case-insensitively: `Work` and `work` are the
    /// same Tag, and attaching an existing one is a no-op. Blank names are
    /// rejected. Tagging does not touch `updated_at` (it is orthogonal to
    /// recency, like pinning).
    pub fn add_tag(&self, id: &str, name: &str) -> Result<Tag> {
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::InvalidTag(name.to_string()));
        }

        let transaction = self.conn.unchecked_transaction()?;
        if load_item(&transaction, id)?.is_none() {
            return Err(Error::ItemNotFound(id.to_string()));
        }

        let canonical: Option<String> = transaction
            .query_row(
                "SELECT name FROM tags WHERE name = ?1 COLLATE NOCASE",
                [name],
                |row| row.get(0),
            )
            .optional()?;
        let canonical = match canonical {
            Some(existing) => existing,
            None => {
                transaction.execute("INSERT INTO tags (name) VALUES (?1)", [name])?;
                name.to_string()
            }
        };

        transaction.execute(
            "INSERT OR IGNORE INTO item_tags (item_id, tag) VALUES (?1, ?2)",
            params![id, canonical],
        )?;
        transaction.commit()?;
        Ok(canonical.into())
    }

    /// Detaches a Tag from a Prompt. Matching is case-insensitive and removing
    /// an absent Tag is a no-op. Tags left attached to no Prompt are pruned.
    pub fn remove_tag(&self, id: &str, name: &str) -> Result<()> {
        let transaction = self.conn.unchecked_transaction()?;
        if load_item(&transaction, id)?.is_none() {
            return Err(Error::ItemNotFound(id.to_string()));
        }

        transaction.execute(
            "DELETE FROM item_tags WHERE item_id = ?1 AND tag COLLATE NOCASE = ?2 COLLATE NOCASE",
            params![id, name],
        )?;
        prune_unused_tags(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list_prompts(&self) -> Result<Vec<ItemSummary>> {
        self.search_prompts("", None)
    }

    /// Searches Prompts by title/body text, optionally narrowed to a Tag. The
    /// text and Tag filters stack, and both paths keep pinned-first ordering.
    pub fn search_prompts(&self, query: &str, tag: Option<&str>) -> Result<Vec<ItemSummary>> {
        match search::plan(query) {
            QueryPlan::All => {
                let mut statement = self.conn.prepare(&format!(
                    "SELECT {SUMMARY_COLUMNS} FROM items i
                     WHERE i.kind = 'prompt' AND i.deleted_at IS NULL
                     {TAG_FILTER}
                     {SUMMARY_ORDER}"
                ))?;
                let rows = statement.query_map(named_params! { ":tag": tag }, summary_tuple)?;
                to_summaries(&self.conn, rows)
            }
            QueryPlan::Trigram(expression) => {
                let mut statement = self.conn.prepare(&format!(
                    "SELECT {SUMMARY_COLUMNS} FROM items_fts JOIN items i ON i.id = items_fts.item_id
                     WHERE items_fts MATCH :expr AND i.kind = 'prompt' AND i.deleted_at IS NULL
                     {TAG_FILTER}
                     {SUMMARY_ORDER}"
                ))?;
                let rows = statement.query_map(
                    named_params! { ":expr": expression, ":tag": tag },
                    summary_tuple,
                )?;
                to_summaries(&self.conn, rows)
            }
            QueryPlan::Like(pattern) => {
                // Keep the LIKE columns in sync with the indexed columns above.
                let mut statement = self.conn.prepare(&format!(
                    "SELECT {SUMMARY_COLUMNS} FROM items i
                     WHERE i.kind = 'prompt' AND i.deleted_at IS NULL
                       AND (i.title LIKE :pattern ESCAPE '\\' OR i.body_md LIKE :pattern ESCAPE '\\')
                     {TAG_FILTER}
                     {SUMMARY_ORDER}"
                ))?;
                let rows = statement.query_map(
                    named_params! { ":pattern": pattern, ":tag": tag },
                    summary_tuple,
                )?;
                to_summaries(&self.conn, rows)
            }
        }
    }

    pub fn soft_delete(&self, id: &str) -> Result<()> {
        let now = self.clock.now_ms();
        let transaction = self.conn.unchecked_transaction()?;
        let changed = transaction.execute(
            "UPDATE items SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id],
        )?;

        if changed == 0 {
            let exists: Option<i64> = transaction
                .query_row("SELECT 1 FROM items WHERE id = ?1", [id], |row| row.get(0))
                .optional()?;
            if exists.is_none() {
                return Err(Error::ItemNotFound(id.to_string()));
            }
        }

        remove_fts(&transaction, id)?;
        remove_variables(&transaction, id)?;
        remove_item_tags(&transaction, id)?;
        prune_unused_tags(&transaction)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn revisions(&self, id: &str) -> Result<Vec<Revision>> {
        let mut statement = self.conn.prepare(
            "SELECT title, body_md, created_at FROM versions
             WHERE item_id = ?1 ORDER BY created_at ASC, id ASC",
        )?;
        let rows = statement.query_map([id], |row| {
            Ok(Revision {
                title: row.get(0)?,
                body_md: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?;

        let mut revisions = Vec::new();
        for row in rows {
            revisions.push(row?);
        }
        Ok(revisions)
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .map_err(Error::from)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn remove_setting(&self, key: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM settings WHERE key = ?1", [key])?;
        Ok(())
    }

    pub fn selected_prompt(&self) -> Result<Option<String>> {
        match self.setting(SETTING_SELECTED_PROMPT)? {
            Some(id) => Ok(self.load(&id)?.map(|item| item.id)),
            None => Ok(None),
        }
    }

    pub fn set_selected_prompt(&self, id: Option<&str>) -> Result<()> {
        match id {
            Some(id) => self.set_setting(SETTING_SELECTED_PROMPT, id),
            None => self.remove_setting(SETTING_SELECTED_PROMPT),
        }
    }
}

struct RawItem {
    id: String,
    kind: String,
    title: String,
    body_md: String,
    pinned: bool,
    created_at: i64,
    updated_at: i64,
    deleted_at: Option<i64>,
}

fn load_item(conn: &Connection, id: &str) -> Result<Option<Item>> {
    let raw = conn
        .query_row(
            &format!("SELECT {ITEM_COLUMNS} FROM items WHERE id = ?1 AND deleted_at IS NULL"),
            [id],
            |row| {
                Ok(RawItem {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    title: row.get(2)?,
                    body_md: row.get(3)?,
                    pinned: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                    deleted_at: row.get(7)?,
                })
            },
        )
        .optional()?;

    raw.map(|raw| {
        Ok(Item {
            id: raw.id,
            kind: ItemKind::from_str(&raw.kind)?,
            title: raw.title,
            body_md: raw.body_md,
            pinned: raw.pinned,
            created_at: raw.created_at,
            updated_at: raw.updated_at,
            deleted_at: raw.deleted_at,
        })
    })
    .transpose()
}

type SummaryTuple = (String, String, String, bool, i64, i64);

fn summary_tuple(row: &rusqlite::Row<'_>) -> rusqlite::Result<SummaryTuple> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn to_summaries(
    conn: &Connection,
    rows: impl Iterator<Item = rusqlite::Result<SummaryTuple>>,
) -> Result<Vec<ItemSummary>> {
    let mut summaries = Vec::new();
    for row in rows {
        let (id, kind, title, pinned, created_at, updated_at) = row?;
        let tags = tags_for(conn, &id)?;
        summaries.push(ItemSummary {
            id,
            kind: ItemKind::from_str(&kind)?,
            title,
            pinned,
            created_at,
            updated_at,
            tags,
        });
    }
    Ok(summaries)
}

/// The Tag names attached to an item, ordered case-insensitively.
fn tags_for(conn: &Connection, id: &str) -> Result<Vec<Tag>> {
    let mut statement = conn
        .prepare("SELECT tag FROM item_tags WHERE item_id = ?1 ORDER BY tag COLLATE NOCASE, tag")?;
    let rows = statement.query_map([id], |row| row.get::<_, String>(0))?;

    let mut tags = Vec::new();
    for row in rows {
        tags.push(Tag::from(row?));
    }
    Ok(tags)
}

fn item_rowid(conn: &Connection, id: &str) -> Result<Option<i64>> {
    conn.query_row("SELECT rowid FROM items WHERE id = ?1", [id], |row| {
        row.get(0)
    })
    .optional()
    .map_err(Error::from)
}

/// Re-indexes an item's `items_fts` row from the content just written, keyed
/// by the item's implicit rowid. Callers pass the fresh content so the index
/// can never lag the row it describes.
fn sync_fts(conn: &Connection, id: &str, title: &str, body_md: &str) -> Result<()> {
    remove_fts(conn, id)?;
    let Some(rowid) = item_rowid(conn, id)? else {
        return Ok(());
    };
    conn.execute(
        "INSERT INTO items_fts (rowid, title, body_md, item_id) VALUES (?1, ?2, ?3, ?4)",
        params![rowid, title, body_md, id],
    )?;
    Ok(())
}

/// Replaces an item's derived Variable definitions from the body just written,
/// so the table can never disagree with the text it describes.
fn sync_variables(conn: &Connection, id: &str, body_md: &str) -> Result<()> {
    remove_variables(conn, id)?;
    for variable in variables::parse(body_md) {
        conn.execute(
            "INSERT INTO variables (item_id, name, default_value) VALUES (?1, ?2, ?3)",
            params![id, variable.name, variable.default_value],
        )?;
    }
    Ok(())
}

fn remove_variables(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM variables WHERE item_id = ?1", [id])?;
    Ok(())
}

fn remove_item_tags(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM item_tags WHERE item_id = ?1", [id])?;
    Ok(())
}

/// Drops Tag vocabulary entries that no Prompt references, so a soft-deleted
/// or re-tagged Prompt leaves no orphan rows behind.
fn prune_unused_tags(conn: &Connection) -> Result<()> {
    conn.execute(
        "DELETE FROM tags
         WHERE NOT EXISTS (SELECT 1 FROM item_tags it WHERE it.tag = tags.name)",
        [],
    )?;
    Ok(())
}

/// Databases written before the repository maintained `variables` have no
/// derived definitions. Parse every live item that has none but whose body
/// could hold a Variable, so the derived table matches the stored text.
fn backfill_variables(conn: &Connection) -> Result<()> {
    let mut statement = conn.prepare(
        "SELECT i.id, i.body_md FROM items i
         WHERE i.deleted_at IS NULL
           AND i.body_md LIKE '%{{%'
           AND NOT EXISTS (SELECT 1 FROM variables v WHERE v.item_id = i.id)",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut pending = Vec::new();
    for row in rows {
        pending.push(row?);
    }
    drop(statement);

    if pending.is_empty() {
        return Ok(());
    }

    let transaction = conn.unchecked_transaction()?;
    for (id, body_md) in pending {
        sync_variables(&transaction, &id, &body_md)?;
    }
    transaction.commit()?;
    Ok(())
}

fn remove_fts(conn: &Connection, id: &str) -> Result<()> {
    if let Some(rowid) = item_rowid(conn, id)? {
        conn.execute("DELETE FROM items_fts WHERE rowid = ?1", [rowid])?;
    }
    Ok(())
}

/// Databases written before the repository maintained `items_fts` have no
/// index rows at all, and a partially written index has fewer rows than live
/// items. Rebuild from the stored items whenever the counts disagree.
fn ensure_fts_index(conn: &Connection) -> Result<()> {
    let live: i64 = conn.query_row(
        "SELECT COUNT(*) FROM items WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    let indexed: i64 = conn.query_row("SELECT COUNT(*) FROM items_fts", [], |row| row.get(0))?;
    if live == indexed {
        return Ok(());
    }

    conn.execute("DELETE FROM items_fts", [])?;
    conn.execute(
        "INSERT INTO items_fts (rowid, title, body_md, item_id)
         SELECT rowid, title, body_md, id FROM items WHERE deleted_at IS NULL",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[derive(Clone)]
    struct TestClock(Rc<Cell<i64>>);

    impl TestClock {
        fn new(start: i64) -> Self {
            Self(Rc::new(Cell::new(start)))
        }

        fn advance(&self, ms: i64) {
            self.0.set(self.0.get() + ms);
        }
    }

    impl Clock for TestClock {
        fn now_ms(&self) -> i64 {
            self.0.get()
        }
    }

    #[derive(Clone)]
    struct TestIds(Rc<Cell<u64>>);

    impl TestIds {
        fn new() -> Self {
            Self(Rc::new(Cell::new(0)))
        }
    }

    impl IdSource for TestIds {
        fn next_id(&self) -> String {
            let next = self.0.get() + 1;
            self.0.set(next);
            format!("item-{next:04}")
        }
    }

    const T0: i64 = 1_700_000_000_000;

    fn library() -> (Library, TestClock, TestIds) {
        let clock = TestClock::new(T0);
        let ids = TestIds::new();
        let library = Library::open_in_memory_with(clock.clone(), ids.clone()).expect("library");
        (library, clock, ids)
    }

    #[test]
    fn create_prompt_persists_an_empty_prompt() {
        let (library, _clock, _ids) = library();
        let created = library.create_prompt().expect("create");

        assert_eq!(created.kind, ItemKind::Prompt);
        assert!(created.title.is_empty());
        assert!(created.body_md.is_empty());
        assert!(!created.pinned);
        assert_eq!(created.created_at, T0);
        assert_eq!(created.updated_at, T0);
        assert_eq!(created.deleted_at, None);
        assert_eq!(library.load(&created.id).expect("load"), Some(created));
    }

    #[test]
    fn save_stores_content_verbatim_and_bumps_updated_at() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        clock.advance(1_000);
        let body = "# 标题\n\n请把这段话翻译成英文：\n\n```rust\nlet x = 1;\n```\n\n中文 🎉";

        let saved = library.save(&item.id, "", body).expect("save");

        assert_eq!(saved.title, "标题");
        assert_eq!(saved.body_md, body);
        assert_eq!(saved.updated_at, T0 + 1_000);
        assert_eq!(
            library.load(&item.id).expect("load").expect("item").body_md,
            body
        );
    }

    #[test]
    fn save_derives_title_only_when_title_is_blank() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");

        clock.advance(1);
        let derived = library
            .save(&item.id, "   ", "# 来自正文\n正文")
            .expect("save");
        assert_eq!(derived.title, "来自正文");

        clock.advance(1);
        let manual = library
            .save(&item.id, "手工标题", "# 来自正文\n正文")
            .expect("save");
        assert_eq!(manual.title, "手工标题");
    }

    #[test]
    fn save_rejects_unknown_items() {
        let (library, _clock, _ids) = library();
        assert!(library.save("missing", "标题", "正文").is_err());
    }

    #[test]
    fn list_prompts_orders_by_most_recent_update() {
        let (library, clock, _ids) = library();
        let first = library.create_prompt().expect("create first");
        clock.advance(1_000);
        let second = library.create_prompt().expect("create second");
        clock.advance(1_000);

        let ids = |items: &[ItemSummary]| {
            items
                .iter()
                .map(|item| item.id.clone())
                .collect::<Vec<String>>()
        };

        assert_eq!(
            ids(&library.list_prompts().expect("list")),
            vec![second.id.clone(), first.id.clone()]
        );

        library.save(&first.id, "first", "body").expect("save");
        assert_eq!(
            ids(&library.list_prompts().expect("list")),
            vec![first.id.clone(), second.id.clone()]
        );
    }

    #[test]
    fn set_pinned_roundtrips_and_leaves_updated_at_alone() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        clock.advance(1_000);

        let pinned = library.set_pinned(&item.id, true).expect("pin");
        assert!(pinned.pinned);
        assert_eq!(pinned.updated_at, item.updated_at);
        assert!(library.load(&item.id).expect("load").expect("item").pinned);

        let unpinned = library.set_pinned(&item.id, false).expect("unpin");
        assert!(!unpinned.pinned);
        assert_eq!(unpinned.updated_at, item.updated_at);
    }

    #[test]
    fn set_pinned_rejects_unknown_and_soft_deleted_items() {
        let (library, _clock, _ids) = library();
        assert!(library.set_pinned("missing", true).is_err());

        let item = library.create_prompt().expect("create");
        library.soft_delete(&item.id).expect("delete");
        assert!(library.set_pinned(&item.id, true).is_err());
    }

    #[test]
    fn list_orders_pinned_prompts_first_then_by_recency() {
        let (library, clock, _ids) = library();
        let oldest = library.create_prompt().expect("create oldest");
        clock.advance(1_000);
        let middle = library.create_prompt().expect("create middle");
        clock.advance(1_000);
        let newest = library.create_prompt().expect("create newest");

        library.set_pinned(&oldest.id, true).expect("pin oldest");
        assert_eq!(
            summary_ids(&library.list_prompts().expect("list")),
            vec![oldest.id.clone(), newest.id.clone(), middle.id.clone()]
        );

        library.set_pinned(&middle.id, true).expect("pin middle");
        assert_eq!(
            summary_ids(&library.list_prompts().expect("list")),
            vec![middle.id.clone(), oldest.id.clone(), newest.id.clone()]
        );

        library.set_pinned(&middle.id, false).expect("unpin middle");
        assert_eq!(
            summary_ids(&library.list_prompts().expect("list")),
            vec![oldest.id.clone(), newest.id.clone(), middle.id.clone()]
        );
    }

    #[test]
    fn search_orders_pinned_prompts_first_in_both_query_paths() {
        let (library, clock, _ids) = library();
        let old = library.create_prompt().expect("create old");
        library
            .save(&old.id, "", "请把这段话翻译成英文")
            .expect("save old");
        clock.advance(1_000);
        let new = library.create_prompt().expect("create new");
        library
            .save(&new.id, "", "请把这句话翻译成法文")
            .expect("save new");

        assert_eq!(
            summary_ids(&library.search_prompts("翻译", None).expect("search like")),
            vec![new.id.clone(), old.id.clone()]
        );

        library.set_pinned(&old.id, true).expect("pin old");

        for query in ["翻译", "翻译成"] {
            assert_eq!(
                summary_ids(&library.search_prompts(query, None).expect("search")),
                vec![old.id.clone(), new.id.clone()],
                "query {query:?} should put the pinned Prompt first"
            );
        }
    }

    #[test]
    fn search_finds_cjk_substrings_from_trigrams() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library
            .save(&item.id, "", "请把这段话翻译成英文")
            .expect("save");

        let hits = library.search_prompts("翻译成", None).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, item.id);
    }

    #[test]
    fn search_falls_back_to_like_for_short_queries() {
        let (library, _clock, _ids) = library();
        let cjk = library.create_prompt().expect("create cjk");
        library
            .save(&cjk.id, "", "请把这段话翻译成英文")
            .expect("save cjk");
        let latin = library.create_prompt().expect("create latin");
        library
            .save(&latin.id, "", "使用 AI 助手整理周报")
            .expect("save latin");

        let short_cjk = library.search_prompts("翻译", None).expect("search 翻译");
        assert_eq!(short_cjk.len(), 1);
        assert_eq!(short_cjk[0].id, cjk.id);

        let short_latin = library.search_prompts("AI", None).expect("search AI");
        assert_eq!(short_latin.len(), 1);
        assert_eq!(short_latin[0].id, latin.id);
    }

    #[test]
    fn search_is_case_insensitive_for_english_queries() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library
            .save(&item.id, "", "Hello World PROMPT")
            .expect("save");

        for query in ["hello world", "HELLO", "he", "world prompt"] {
            let hits = library.search_prompts(query, None).expect("search");
            assert_eq!(hits.len(), 1, "query {query:?} should match");
            assert_eq!(hits[0].id, item.id);
        }
    }

    fn summary_ids(items: &[ItemSummary]) -> Vec<String> {
        items.iter().map(|item| item.id.clone()).collect()
    }

    #[test]
    fn search_matches_titles_and_bodies() {
        let (library, clock, _ids) = library();
        let title_item = library.create_prompt().expect("create title item");
        library
            .save(&title_item.id, "模板标题示例", "正文无关")
            .expect("save title item");
        clock.advance(1_000);
        let body_item = library.create_prompt().expect("create body item");
        library
            .save(&body_item.id, "无关标题", "这里包含正文短语")
            .expect("save body item");

        let hits = library
            .search_prompts("模板标题", None)
            .expect("search title");
        assert_eq!(summary_ids(&hits), vec![title_item.id.clone()]);

        let hits = library
            .search_prompts("正文短语", None)
            .expect("search body");
        assert_eq!(summary_ids(&hits), vec![body_item.id.clone()]);

        let hits = library
            .search_prompts("模板", None)
            .expect("search short title");
        assert_eq!(summary_ids(&hits), vec![title_item.id.clone()]);
    }

    #[test]
    fn search_keeps_the_fts_index_in_sync_on_save() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library.save(&item.id, "", "alpha content").expect("save");
        assert_eq!(
            library.search_prompts("alpha", None).expect("search").len(),
            1
        );

        clock.advance(REVISION_WINDOW_MS);
        library.save(&item.id, "", "beta content").expect("save");

        assert!(
            library
                .search_prompts("alpha", None)
                .expect("search")
                .is_empty()
        );
        assert_eq!(
            library.search_prompts("beta", None).expect("search").len(),
            1
        );
    }

    #[test]
    fn search_excludes_soft_deleted_items_from_both_query_paths() {
        let (library, _clock, _ids) = library();
        let kept = library.create_prompt().expect("create kept");
        library
            .save(&kept.id, "", "请把这段话翻译成英文")
            .expect("save kept");
        let removed = library.create_prompt().expect("create removed");
        library
            .save(&removed.id, "", "请把这句话翻译成法文")
            .expect("save removed");

        for query in ["翻译", "翻译成"] {
            assert_eq!(
                library.search_prompts(query, None).expect("search").len(),
                2,
                "query {query:?} should match both items before deletion"
            );
        }

        library.soft_delete(&removed.id).expect("delete");

        for query in ["翻译", "翻译成"] {
            let hits = library.search_prompts(query, None).expect("search");
            assert_eq!(summary_ids(&hits), vec![kept.id.clone()], "query {query:?}");
        }
        assert!(
            library
                .search_prompts("成法文", None)
                .expect("search")
                .is_empty()
        );
    }

    #[test]
    fn blank_search_lists_every_prompt_in_update_order() {
        let (library, clock, _ids) = library();
        let first = library.create_prompt().expect("create first");
        clock.advance(1_000);
        let second = library.create_prompt().expect("create second");
        library.save(&first.id, "first", "one").expect("save first");
        clock.advance(1_000);
        library
            .save(&second.id, "second", "two")
            .expect("save second");

        let hits = library.search_prompts("  ", None).expect("search");
        assert_eq!(summary_ids(&hits), vec![second.id, first.id]);
    }

    #[test]
    fn search_treats_query_syntax_literally() {
        let (library, _clock, _ids) = library();
        let percent = library.create_prompt().expect("create percent");
        library
            .save(&percent.id, "", "进度 100% 完成")
            .expect("save percent");
        let underscore = library.create_prompt().expect("create underscore");
        library
            .save(&underscore.id, "", "snake_case 命名")
            .expect("save underscore");
        let quoted = library.create_prompt().expect("create quoted");
        library
            .save(&quoted.id, "", "say \"hi\" now")
            .expect("save quoted");

        let hits = library.search_prompts("%", None).expect("search percent");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, percent.id);

        let hits = library
            .search_prompts("_", None)
            .expect("search underscore");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, underscore.id);

        let hits = library
            .search_prompts("\"hi\"", None)
            .expect("search quoted");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, quoted.id);
    }

    #[test]
    fn opening_rebuilds_a_missing_fts_index_from_stored_items() {
        let conn = crate::storage::open_in_memory().expect("open");
        crate::storage::migrate(&conn).expect("migrate");
        conn.execute(
            "INSERT INTO items (id, kind, title, body_md, created_at, updated_at)
             VALUES ('legacy', 'prompt', '周报整理', '请把这段话翻译成英文', ?1, ?1)",
            [T0],
        )
        .expect("insert legacy item");

        let library =
            Library::from_connection(conn, TestClock::new(T0), TestIds::new()).expect("library");

        let hits = library.search_prompts("翻译成", None).expect("search");
        assert_eq!(summary_ids(&hits), vec!["legacy"]);
    }

    #[test]
    fn opening_rebuilds_a_partially_indexed_fts_table() {
        let conn = crate::storage::open_in_memory().expect("open");
        crate::storage::migrate(&conn).expect("migrate");
        conn.execute(
            "INSERT INTO items (id, kind, title, body_md, created_at, updated_at)
             VALUES ('indexed', 'prompt', '已索引', '请把这段话翻译成英文', ?1, ?1)",
            [T0],
        )
        .expect("insert indexed item");
        conn.execute(
            "INSERT INTO items (id, kind, title, body_md, created_at, updated_at)
             VALUES ('missing', 'prompt', '未索引', '请把这句话翻译成法文', ?1, ?1)",
            [T0],
        )
        .expect("insert unindexed item");
        conn.execute(
            "INSERT INTO items_fts (rowid, title, body_md, item_id)
             SELECT rowid, title, body_md, id FROM items WHERE id = 'indexed'",
            [],
        )
        .expect("seed partial index");

        let library =
            Library::from_connection(conn, TestClock::new(T0), TestIds::new()).expect("library");

        let hits = library.search_prompts("翻译成", None).expect("search");
        assert_eq!(summary_ids(&hits).len(), 2);
    }

    fn variable(name: &str, default_value: Option<&str>) -> VariableDef {
        VariableDef {
            name: name.to_string(),
            default_value: default_value.map(str::to_string),
        }
    }

    #[test]
    fn save_derives_variables_merging_duplicates() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");

        library
            .save(
                &item.id,
                "",
                "请把 {{text:这段话}} 翻译成 {{lang}}，{{text:别的}}",
            )
            .expect("save");

        assert_eq!(
            library.variables(&item.id).expect("variables"),
            vec![variable("lang", None), variable("text", Some("这段话"))]
        );
    }

    #[test]
    fn save_derives_empty_and_multiline_defaults() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");

        library
            .save(&item.id, "", "{{empty:}} 与 {{multi:第一行\n第二行}}")
            .expect("save");

        assert_eq!(
            library.variables(&item.id).expect("variables"),
            vec![
                variable("empty", Some("")),
                variable("multi", Some("第一行\n第二行")),
            ]
        );
    }

    #[test]
    fn save_ignores_escaped_braces_when_deriving_variables() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");

        library
            .save(&item.id, "", r"\{{not-a-variable}} but {{real}}")
            .expect("save");

        assert_eq!(
            library.variables(&item.id).expect("variables"),
            vec![variable("real", None)]
        );
    }

    #[test]
    fn save_replaces_stale_variables() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library.save(&item.id, "", "{{old}}").expect("save old");
        assert_eq!(library.variables(&item.id).expect("variables").len(), 1);

        clock.advance(REVISION_WINDOW_MS);
        library.save(&item.id, "", "没有变量了").expect("save new");

        assert!(library.variables(&item.id).expect("variables").is_empty());
    }

    #[test]
    fn soft_delete_clears_derived_variables() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library.save(&item.id, "", "{{a:1}}").expect("save");

        library.soft_delete(&item.id).expect("delete");

        assert!(library.variables(&item.id).expect("variables").is_empty());
    }

    #[test]
    fn opening_backfills_variables_for_pre_existing_items() {
        let conn = crate::storage::open_in_memory().expect("open");
        crate::storage::migrate(&conn).expect("migrate");
        conn.execute(
            "INSERT INTO items (id, kind, title, body_md, created_at, updated_at)
             VALUES ('legacy', 'prompt', '旧条目', '你好 {{name:世界}} {{keep}}', ?1, ?1)",
            [T0],
        )
        .expect("insert legacy item");
        conn.execute(
            "INSERT INTO items (id, kind, title, body_md, created_at, updated_at)
             VALUES ('plain', 'prompt', '无变量', '普通正文', ?1, ?1)",
            [T0],
        )
        .expect("insert plain item");

        let library =
            Library::from_connection(conn, TestClock::new(T0), TestIds::new()).expect("library");

        assert_eq!(
            library.variables("legacy").expect("variables"),
            vec![variable("keep", None), variable("name", Some("世界"))]
        );
        assert!(library.variables("plain").expect("variables").is_empty());
    }

    #[test]
    fn soft_delete_hides_items_from_load_and_list() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        clock.advance(1_000);

        library.soft_delete(&item.id).expect("delete");

        assert_eq!(library.load(&item.id).expect("load"), None);
        assert!(library.list_prompts().expect("list").is_empty());

        library.soft_delete(&item.id).expect("delete is idempotent");
        assert!(library.soft_delete("missing").is_err());
    }

    #[test]
    fn first_save_writes_one_revision_immediately() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");

        library.save(&item.id, "", "hello").expect("save");

        assert_eq!(library.revisions(&item.id).expect("revisions").len(), 1);
    }

    #[test]
    fn identical_content_is_not_saved_again() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        let first = library.save(&item.id, "", "hello").expect("save");

        clock.advance(60_000);
        let second = library.save(&item.id, "", "hello").expect("save");

        assert_eq!(second.updated_at, first.updated_at);
        assert_eq!(library.revisions(&item.id).expect("revisions").len(), 1);
    }

    #[test]
    fn revisions_are_rate_limited_to_the_ten_second_window() {
        let (library, clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library.save(&item.id, "", "one").expect("save");

        clock.advance(REVISION_WINDOW_MS - 1);
        library.save(&item.id, "", "two").expect("save");
        assert_eq!(library.revisions(&item.id).expect("revisions").len(), 1);

        clock.advance(1);
        library.save(&item.id, "", "three").expect("save");
        let revisions = library.revisions(&item.id).expect("revisions");
        assert_eq!(revisions.len(), 2);
        assert_eq!(revisions[1].body_md, "three");
    }

    #[test]
    fn revisions_are_scoped_to_their_item() {
        let (library, clock, _ids) = library();
        let first = library.create_prompt().expect("create");
        let second = library.create_prompt().expect("create");

        library.save(&first.id, "", "one").expect("save");
        clock.advance(REVISION_WINDOW_MS);
        library.save(&second.id, "", "two").expect("save");

        assert_eq!(library.revisions(&first.id).expect("revisions").len(), 1);
        assert_eq!(library.revisions(&second.id).expect("revisions").len(), 1);
    }

    #[test]
    fn selected_prompt_roundtrips_and_clears_when_deleted() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        assert_eq!(library.selected_prompt().expect("selected"), None);

        library.set_selected_prompt(Some(&item.id)).expect("select");
        assert_eq!(
            library.selected_prompt().expect("selected"),
            Some(item.id.clone())
        );

        library.soft_delete(&item.id).expect("delete");
        assert_eq!(library.selected_prompt().expect("selected"), None);

        library.set_selected_prompt(None).expect("clear");
        assert_eq!(
            library.setting(SETTING_SELECTED_PROMPT).expect("setting"),
            None
        );
    }

    #[test]
    fn settings_roundtrip_upsert_and_remove() {
        let (library, _clock, _ids) = library();

        assert_eq!(library.setting(SETTING_THEME).expect("setting"), None);
        library.set_setting(SETTING_THEME, "dark").expect("set");
        library.set_setting(SETTING_THEME, "light").expect("update");
        assert_eq!(
            library.setting(SETTING_THEME).expect("setting"),
            Some("light".to_string())
        );

        library.remove_setting(SETTING_THEME).expect("remove");
        assert_eq!(library.setting(SETTING_THEME).expect("setting"), None);
    }

    #[test]
    fn pinned_state_survives_reopening_the_database() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "promptdeck-pinned-{}-{stamp}.sqlite3",
            std::process::id()
        ));

        let clock = TestClock::new(T0);
        let ids = TestIds::new();
        {
            let library = Library::open_with(&path, clock.clone(), ids.clone()).expect("open");
            let first = library.create_prompt().expect("first");
            clock.advance(1_000);
            library.create_prompt().expect("second");
            library.set_pinned(&first.id, true).expect("pin");
        }

        let reopened = Library::open_with(&path, clock, ids).expect("reopen");
        let listed = reopened.list_prompts().expect("list");
        assert_eq!(
            summary_ids(&listed),
            vec!["item-0001".to_string(), "item-0002".to_string()]
        );
        assert!(listed[0].pinned);
        drop(reopened);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }

    fn tag(name: &str) -> Tag {
        Tag::new(name)
    }

    #[test]
    fn add_tag_is_case_insensitive_unique_and_roundtrips() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");

        assert_eq!(
            library.add_tag(&item.id, "  Work  ").expect("add").as_str(),
            "Work"
        );
        assert_eq!(
            library.add_tag(&item.id, "work").expect("dup").as_str(),
            "Work"
        );
        assert_eq!(library.tags(&item.id).expect("tags"), vec![tag("Work")]);
        assert_eq!(
            library
                .load(&item.id)
                .expect("load")
                .expect("item")
                .updated_at,
            T0,
            "tagging is orthogonal to recency"
        );
    }

    #[test]
    fn add_tag_rejects_blank_names_and_unknown_items() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");

        assert!(library.add_tag(&item.id, "   ").is_err());
        assert!(library.add_tag("missing", "work").is_err());

        library.soft_delete(&item.id).expect("delete");
        assert!(library.add_tag(&item.id, "work").is_err());
    }

    #[test]
    fn tags_are_listed_case_insensitively_sorted() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        for tag in ["beta", "Work", "alpha"] {
            library.add_tag(&item.id, tag).expect("add");
        }

        assert_eq!(
            library.tags(&item.id).expect("tags"),
            vec![tag("alpha"), tag("beta"), tag("Work")]
        );
    }

    #[test]
    fn remove_tag_detaches_case_insensitively_and_is_idempotent() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library.add_tag(&item.id, "Work").expect("add");
        library.add_tag(&item.id, "beta").expect("add");

        library.remove_tag(&item.id, "work").expect("remove");
        assert_eq!(library.tags(&item.id).expect("tags"), vec![tag("beta")]);

        library.remove_tag(&item.id, "work").expect("remove again");
        assert_eq!(library.tags(&item.id).expect("tags"), vec![tag("beta")]);
    }

    #[test]
    fn search_with_a_tag_keeps_only_tagged_prompts_in_order() {
        let (library, clock, _ids) = library();
        let tagged_old = library.create_prompt().expect("create");
        library.add_tag(&tagged_old.id, "work").expect("tag");
        clock.advance(1_000);
        let untagged = library.create_prompt().expect("create");
        clock.advance(1_000);
        let tagged_new = library.create_prompt().expect("create");
        library.add_tag(&tagged_new.id, "work").expect("tag");

        assert_eq!(
            summary_ids(&library.search_prompts("", Some("work")).expect("filter")),
            vec![tagged_new.id.clone(), tagged_old.id.clone()]
        );
        assert_eq!(library.search_prompts("", None).expect("all").len(), 3);
        assert!(untagged.id != tagged_old.id);
    }

    #[test]
    fn tag_filter_is_case_insensitive_and_keeps_pinned_first() {
        let (library, clock, _ids) = library();
        let older = library.create_prompt().expect("create");
        library.save(&older.id, "", "alpha body").expect("save");
        library.add_tag(&older.id, "Work").expect("tag");
        clock.advance(1_000);
        let newer = library.create_prompt().expect("create");
        library.save(&newer.id, "", "alpha body two").expect("save");
        library.add_tag(&newer.id, "work").expect("tag");

        assert_eq!(
            summary_ids(
                &library
                    .search_prompts("alpha", Some("WORK"))
                    .expect("filter")
            ),
            vec![newer.id.clone(), older.id.clone()]
        );

        library.set_pinned(&older.id, true).expect("pin");
        assert_eq!(
            summary_ids(
                &library
                    .search_prompts("alpha", Some("work"))
                    .expect("filter")
            ),
            vec![older.id.clone(), newer.id.clone()]
        );
    }

    #[test]
    fn tag_filter_combines_with_search_text_on_both_query_paths() {
        let (library, clock, _ids) = library();
        let match_item = library.create_prompt().expect("create");
        library
            .save(&match_item.id, "", "请把这段话翻译成英文")
            .expect("save");
        library.add_tag(&match_item.id, "work").expect("tag");
        clock.advance(1_000);
        let other_tag = library.create_prompt().expect("create");
        library
            .save(&other_tag.id, "", "请把这段话翻译成法文")
            .expect("save");
        library.add_tag(&other_tag.id, "personal").expect("tag");
        clock.advance(1_000);
        let other_text = library.create_prompt().expect("create");
        library.save(&other_text.id, "", "整理周报").expect("save");
        library.add_tag(&other_text.id, "work").expect("tag");

        // "翻译" is short → LIKE path; "翻译成" is 3 chars → FTS path.
        for query in ["翻译", "翻译成"] {
            assert_eq!(
                summary_ids(&library.search_prompts(query, Some("work")).expect("filter")),
                vec![match_item.id.clone()],
                "query {query:?} with tag filter"
            );
        }
    }

    #[test]
    fn soft_delete_cascades_tag_links_and_prunes_unused_tags() {
        let (library, _clock, _ids) = library();
        let item = library.create_prompt().expect("create");
        library.add_tag(&item.id, "work").expect("tag");
        library.add_tag(&item.id, "shared").expect("tag");
        let survivor = library.create_prompt().expect("create");
        library.add_tag(&survivor.id, "shared").expect("tag");

        library.soft_delete(&item.id).expect("delete");

        let links: i64 = library
            .conn
            .query_row("SELECT COUNT(*) FROM item_tags", [], |row| row.get(0))
            .expect("count links");
        assert_eq!(links, 1, "only the surviving Prompt keeps its link");
        let orphan: i64 = library
            .conn
            .query_row("SELECT COUNT(*) FROM tags WHERE name = 'work'", [], |row| {
                row.get(0)
            })
            .expect("count orphan tag");
        assert_eq!(orphan, 0, "unused tag vocabulary is pruned");

        assert!(
            library
                .search_prompts("", Some("work"))
                .expect("filter")
                .is_empty()
        );
        assert_eq!(
            summary_ids(&library.search_prompts("", Some("shared")).expect("filter")),
            vec![survivor.id.clone()]
        );
    }

    #[test]
    fn tag_links_and_filter_survive_reopening_the_database() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "promptdeck-tags-{}-{stamp}.sqlite3",
            std::process::id()
        ));

        let clock = TestClock::new(T0);
        let ids = TestIds::new();
        let item_id;
        {
            let library = Library::open_with(&path, clock.clone(), ids.clone()).expect("open");
            let item = library.create_prompt().expect("create");
            item_id = item.id.clone();
            library.add_tag(&item.id, "Work").expect("tag");
        }

        let reopened = Library::open_with(&path, clock, ids).expect("reopen");
        assert_eq!(reopened.tags(&item_id).expect("tags"), vec![tag("Work")]);
        assert_eq!(
            summary_ids(&reopened.search_prompts("", Some("work")).expect("filter")),
            vec![item_id.clone()]
        );
        assert_eq!(
            reopened
                .list_prompts()
                .expect("list")
                .first()
                .expect("row")
                .tags,
            vec![tag("Work")]
        );
        drop(reopened);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(format!("{}-wal", path.display()));
        let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    }
}
