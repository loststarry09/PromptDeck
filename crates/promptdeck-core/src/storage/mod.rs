use std::path::Path;

use rusqlite::Connection;

pub mod library;

pub const MIGRATIONS: &[&str] = &["CREATE TABLE items (
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
     );"];

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    configure(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    Ok(conn)
}

fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;",
    )
}

pub fn schema_version(conn: &Connection) -> rusqlite::Result<usize> {
    conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map(|version| version as usize)
}

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let current = schema_version(conn)?;
    let pending = &MIGRATIONS[current.min(MIGRATIONS.len())..];
    if pending.is_empty() {
        return Ok(());
    }

    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        for migration in pending {
            conn.execute_batch(migration)?;
        }
        conn.execute_batch(&format!("PRAGMA user_version = {}", MIGRATIONS.len()))
    })();

    match result {
        Ok(()) => conn.execute_batch("COMMIT"),
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table_names(conn: &Connection) -> Vec<String> {
        let mut statement = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("prepare");
        let names = statement
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect");
        names
            .into_iter()
            .filter(|name| !name.starts_with("sqlite_"))
            .collect()
    }

    #[test]
    fn migrate_creates_v1_schema_and_is_idempotent() {
        let conn = open_in_memory().expect("open");
        assert_eq!(schema_version(&conn).expect("version"), 0);

        migrate(&conn).expect("first migrate");
        assert_eq!(schema_version(&conn).expect("version"), MIGRATIONS.len());

        let names = table_names(&conn);
        for expected in [
            "items",
            "tags",
            "item_tags",
            "variables",
            "versions",
            "settings",
        ] {
            assert!(names.contains(&expected.to_string()), "missing {expected}");
        }
        assert!(
            names.iter().any(|name| name == "items_fts"),
            "missing items_fts"
        );

        migrate(&conn).expect("second migrate");
        assert_eq!(schema_version(&conn).expect("version"), MIGRATIONS.len());

        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)",
            ("theme", "dark"),
        )
        .expect("insert");
        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme'",
                [],
                |row| row.get(0),
            )
            .expect("select");
        assert_eq!(value, "dark");
    }

    #[test]
    fn migrate_rejects_item_kinds_outside_prompt_and_block() {
        let conn = open_in_memory().expect("open");
        migrate(&conn).expect("migrate");

        let result = conn.execute(
            "INSERT INTO items (id, kind, title, body_md, created_at, updated_at)
             VALUES ('1', 'mystery', '', '', 0, 0)",
            [],
        );

        assert!(result.is_err());
    }

    #[test]
    fn bundled_sqlite_has_fts5() {
        let conn = open_in_memory().expect("open");
        conn.execute_batch("CREATE VIRTUAL TABLE probe USING fts5(content);")
            .expect("create fts5 table");
        conn.execute(
            "INSERT INTO probe (content) VALUES ('PromptDeck full text search')",
            [],
        )
        .expect("insert");
        let matches: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM probe WHERE probe MATCH 'promptdeck'",
                [],
                |row| row.get(0),
            )
            .expect("match");
        assert_eq!(matches, 1);
    }

    #[test]
    fn bundled_sqlite_fts5_trigram_matches_cjk_and_latin_substrings() {
        let conn = open_in_memory().expect("open");
        conn.execute_batch("CREATE VIRTUAL TABLE probe USING fts5(content, tokenize = 'trigram');")
            .expect("create trigram table");
        conn.execute(
            "INSERT INTO probe (content) VALUES
                 ('请帮我把这段话翻译成英文'),
                 ('PromptDeck supports substring search')",
            [],
        )
        .expect("insert");

        let cjk: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM probe WHERE probe MATCH '翻译成'",
                [],
                |row| row.get(0),
            )
            .expect("cjk match");
        assert_eq!(cjk, 1);

        let latin: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM probe WHERE probe MATCH 'SUBSTRING'",
                [],
                |row| row.get(0),
            )
            .expect("latin match");
        assert_eq!(latin, 1);

        let too_short: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM probe WHERE probe MATCH '翻译'",
                [],
                |row| row.get(0),
            )
            .expect("short query must not error");
        assert_eq!(too_short, 0);

        let like_fallback: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM probe WHERE content LIKE '%翻译%'",
                [],
                |row| row.get(0),
            )
            .expect("like fallback");
        assert_eq!(like_fallback, 1);
    }
}
