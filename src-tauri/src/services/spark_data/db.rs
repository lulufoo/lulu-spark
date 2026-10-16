//! `spark-data.sqlite`: durable metadata (notes, tags, knowledge docs).
//!
//! No process-wide connection: like `search_index` and the agent session store,
//! every operation calls [`open`] and gets its own `Connection`. Writes are
//! serialized by SQLite itself (WAL allows a single writer; `busy_timeout` makes
//! a second writer wait instead of failing). The schema version lives in
//! `PRAGMA user_version`; each entry of [`MIGRATIONS`] upgrades the file by one
//! version.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use crate::config::paths;

pub const DB_FILE: &str = "spark-data.sqlite";

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// `MIGRATIONS[n]` upgrades a file from `user_version = n` to `n + 1`.
/// Append a step to change the schema; never edit an existing one.
const MIGRATIONS: [&str; 2] = [r#"
CREATE TABLE IF NOT EXISTS note (
  doc_id       TEXT PRIMARY KEY,
  raw_path     TEXT NOT NULL,
  digest_path  TEXT,
  source_type  TEXT,
  created_at   TEXT,
  importance   TEXT CHECK (importance IS NULL OR importance IN ('high', 'medium', 'low')),
  done         INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
  links        TEXT NOT NULL DEFAULT '[]' CHECK (json_type(links) = 'array'),
  comments     TEXT NOT NULL DEFAULT '[]' CHECK (json_type(comments) = 'array'),
  tags         TEXT NOT NULL DEFAULT '[]' CHECK (json_type(tags) = 'array'),
  highlights   TEXT NOT NULL DEFAULT '[]' CHECK (json_type(highlights) = 'array'),
  translations TEXT NOT NULL DEFAULT '[]' CHECK (json_type(translations) = 'array'),
  title        TEXT
);
CREATE INDEX IF NOT EXISTS note_raw_path ON note(raw_path);

CREATE TABLE IF NOT EXISTS tag (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS knowledge_doc (
  doc_id     TEXT PRIMARY KEY,
  repo       TEXT NOT NULL,
  path       TEXT NOT NULL,
  status     TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'missing')),
  comments   TEXT NOT NULL DEFAULT '[]' CHECK (json_type(comments) = 'array'),
  links      TEXT NOT NULL DEFAULT '[]' CHECK (json_type(links) = 'array'),
  highlights TEXT NOT NULL DEFAULT '[]' CHECK (json_type(highlights) = 'array'),
  UNIQUE (repo, path)
);
"#, r#"
CREATE TABLE note_category (
  id TEXT PRIMARY KEY,
  folder TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT ''
);
"#];

pub fn db_path() -> Result<PathBuf, String> {
    paths::spark_data_db_path().map_err(|e| format!("{e:?}"))
}

/// Open the production database, creating or upgrading it as needed.
pub fn open() -> Result<Connection, String> {
    open_at(&db_path()?)
}

pub fn open_at(path: &Path) -> Result<Connection, String> {
    open_with_steps(path, &MIGRATIONS)
}

pub(crate) fn open_with_steps(path: &Path, steps: &[&str]) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.busy_timeout(BUSY_TIMEOUT).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| e.to_string())?;
    migrate(&conn, steps)?;
    Ok(conn)
}

/// Run every step from the file's `user_version` up to `steps.len()`, each in its
/// own transaction. A file newer than this build is refused untouched.
pub(crate) fn migrate(conn: &Connection, steps: &[&str]) -> Result<(), String> {
    let version: usize = conn
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())? as usize;
    if version > steps.len() {
        return Err(format!(
            "{DB_FILE} is at schema version {version}, newer than this build ({})",
            steps.len()
        ));
    }
    for (from, step) in steps.iter().enumerate().skip(version) {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(step).map_err(|e| e.to_string())?;
        tx.pragma_update(None, "user_version", from + 1)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    Ok(())
}
