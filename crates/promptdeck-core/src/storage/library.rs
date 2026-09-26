use std::path::Path;
use std::str::FromStr;

use rusqlite::{Connection, OptionalExtension, params};

use crate::clock::{Clock, SystemClock};
use crate::error::{Error, Result};
use crate::id::{IdSource, UuidV7Ids};
use crate::model::{Item, ItemKind, ItemSummary, Revision, content_hash, derive_title};

pub const REVISION_WINDOW_MS: i64 = 10_000;
pub const SETTING_THEME: &str = "theme";
pub const SETTING_SELECTED_PROMPT: &str = "library.selected_prompt";
pub const SETTING_RAIL_EXPANDED: &str = "rail.expanded";

const ITEM_COLUMNS: &str = "id, kind, title, body_md, pinned, created_at, updated_at, deleted_at";

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

        self.conn.execute(
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

    pub fn list_prompts(&self) -> Result<Vec<ItemSummary>> {
        let mut statement = self.conn.prepare(
            "SELECT id, kind, title, pinned, created_at, updated_at FROM items
             WHERE kind = 'prompt' AND deleted_at IS NULL
             ORDER BY updated_at DESC, created_at DESC, id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?;

        let mut summaries = Vec::new();
        for row in rows {
            let (id, kind, title, pinned, created_at, updated_at) = row?;
            summaries.push(ItemSummary {
                id,
                kind: ItemKind::from_str(&kind)?,
                title,
                pinned,
                created_at,
                updated_at,
            });
        }
        Ok(summaries)
    }

    pub fn soft_delete(&self, id: &str) -> Result<()> {
        let now = self.clock.now_ms();
        let changed = self.conn.execute(
            "UPDATE items SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now, id],
        )?;

        if changed == 0 {
            let exists: Option<i64> = self
                .conn
                .query_row("SELECT 1 FROM items WHERE id = ?1", [id], |row| row.get(0))
                .optional()?;
            if exists.is_none() {
                return Err(Error::ItemNotFound(id.to_string()));
            }
        }
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
}
