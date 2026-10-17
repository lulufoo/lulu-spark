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

use super::legacy_rows_step::{DROP_LEGACY_STEP, LEGACY_ROWS_STEP};
use crate::config::paths;

pub const DB_FILE: &str = "spark-data.sqlite";

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Columns of every per-workspace document table (`doc_<workspace id>`).
///
/// A macro so one literal feeds both the static migration step below (via
/// `concat!`) and the runtime table template in `doc_table.rs`. Same columns as
/// `note`, with `category_id` replaced by `project_id` and no `status` (the
/// scan-and-reconcile mode is retired).
macro_rules! doc_table_columns {
    () => {
        "doc_id       TEXT PRIMARY KEY,
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
  title        TEXT,
  project_id   TEXT NOT NULL REFERENCES project(id)"
    };
}
pub(crate) use doc_table_columns;

/// DDL for one document table with a literal name.
macro_rules! doc_table_sql {
    ($table:literal) => {
        concat!(
            "CREATE TABLE ",
            $table,
            " (",
            doc_table_columns!(),
            ");\nCREATE INDEX ",
            $table,
            "_raw_path ON ",
            $table,
            "(raw_path);\n"
        )
    };
}

/// `MIGRATIONS[n]` upgrades a file from `user_version = n` to `n + 1`.
/// Append a step to change the schema; never edit an existing one.
/// Step 2 rebuilds `note` instead of `ADD COLUMN … REFERENCES`: SQLite rejects a
/// NOT NULL default plus FK when the table already has rows.
/// Step 6 adds the workspace / project containers: empty tables plus the two seed
/// registry rows. Step 7 copies the legacy rows into them and step 8 drops the legacy
/// tables (both in `legacy_rows_step`). Step 9 adds `disabled_at` to `workspace` and
/// `project`: a workspace or project is disabled (hidden, data kept), never deleted.
/// Step 10 adds `icon` (a built-in icon id, NULL = default) to both tables.
/// Step 11 adds `sort_index` (the user's order) to both tables, filled from the old order:
/// the default workspace first then by name, projects by title within their workspace.
/// Step 12 adds `is_inbox` (one built-in project per workspace). Rows that already use
/// folder `inbox` are marked; missing inboxes are not inserted here.
pub(crate) const MIGRATIONS: [&str; 13] = [r#"
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
"#, r#"
INSERT OR IGNORE INTO note_category(id, folder, title, description) VALUES ('inbox', 'inbox', 'Inbox', '');
CREATE TABLE note__with_category (
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
  title        TEXT,
  category_id  TEXT NOT NULL DEFAULT 'inbox' REFERENCES note_category(id)
);
INSERT INTO note__with_category (
  doc_id, raw_path, digest_path, source_type, created_at, importance, done,
  links, comments, tags, highlights, translations, title, category_id
)
SELECT
  doc_id, raw_path, digest_path, source_type, created_at, importance, done,
  links, comments, tags, highlights, translations, title, 'inbox'
FROM note;
DROP TABLE note;
ALTER TABLE note__with_category RENAME TO note;
CREATE INDEX IF NOT EXISTS note_raw_path ON note(raw_path);
"#, r#"
CREATE TABLE knowledge_directory (
  id TEXT PRIMARY KEY,
  folder TEXT NOT NULL UNIQUE,
  category TEXT NOT NULL DEFAULT 'Uncategorized' CHECK (category <> ''),
  description TEXT NOT NULL DEFAULT ''
);
"#, r#"
DELETE FROM knowledge_doc WHERE repo IN ('ai-dev-docs', 'lulu-workflow-framework');
INSERT INTO knowledge_directory (id, folder, category, description)
SELECT repo, repo, 'Uncategorized', ''
FROM knowledge_doc
WHERE repo NOT IN (SELECT folder FROM knowledge_directory)
GROUP BY repo;
CREATE TABLE knowledge_doc__with_dir (
  doc_id       TEXT PRIMARY KEY,
  repo         TEXT NOT NULL,
  path         TEXT NOT NULL,
  directory_id TEXT NOT NULL REFERENCES knowledge_directory(id),
  status       TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'missing')),
  comments     TEXT NOT NULL DEFAULT '[]' CHECK (json_type(comments) = 'array'),
  links        TEXT NOT NULL DEFAULT '[]' CHECK (json_type(links) = 'array'),
  highlights   TEXT NOT NULL DEFAULT '[]' CHECK (json_type(highlights) = 'array'),
  UNIQUE (repo, path)
);
INSERT INTO knowledge_doc__with_dir (
  doc_id, repo, path, directory_id, status, comments, links, highlights
)
SELECT d.doc_id, d.repo, d.path, k.id, d.status, d.comments, d.links, d.highlights
FROM knowledge_doc d
INNER JOIN knowledge_directory k ON k.folder = d.repo;
DROP TABLE knowledge_doc;
ALTER TABLE knowledge_doc__with_dir RENAME TO knowledge_doc;
"#, r#"
CREATE TABLE knowledge_doc__no_repo (
  doc_id       TEXT PRIMARY KEY,
  path         TEXT NOT NULL,
  directory_id TEXT NOT NULL REFERENCES knowledge_directory(id),
  status       TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'missing')),
  comments     TEXT NOT NULL DEFAULT '[]' CHECK (json_type(comments) = 'array'),
  links        TEXT NOT NULL DEFAULT '[]' CHECK (json_type(links) = 'array'),
  highlights   TEXT NOT NULL DEFAULT '[]' CHECK (json_type(highlights) = 'array'),
  UNIQUE (directory_id, path)
);
INSERT INTO knowledge_doc__no_repo (
  doc_id, path, directory_id, status, comments, links, highlights
)
SELECT doc_id, path, directory_id, status, comments, links, highlights
FROM knowledge_doc;
DROP TABLE knowledge_doc;
ALTER TABLE knowledge_doc__no_repo RENAME TO knowledge_doc;
"#, concat!(r#"
CREATE TABLE workspace (
  id         TEXT PRIMARY KEY CHECK (id <> '' AND id NOT GLOB '*[^a-z0-9_]*'),
  name       TEXT NOT NULL UNIQUE CHECK (name <> ''),
  doc_table  TEXT NOT NULL UNIQUE CHECK (doc_table = 'doc_' || id),
  view_mode  TEXT NOT NULL DEFAULT 'directory' CHECK (view_mode IN ('directory', 'timeline')),
  is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
  created_at TEXT
);
CREATE UNIQUE INDEX workspace_one_default ON workspace(is_default) WHERE is_default = 1;
CREATE TRIGGER workspace_keys_immutable BEFORE UPDATE OF id, doc_table, is_default ON workspace
BEGIN SELECT RAISE(ABORT, 'workspace id, doc_table and is_default cannot change'); END;
CREATE TRIGGER workspace_default_undeletable BEFORE DELETE ON workspace WHEN OLD.is_default = 1
BEGIN SELECT RAISE(ABORT, 'the default workspace cannot be deleted'); END;
CREATE TABLE project (
  id           TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspace(id),
  folder       TEXT NOT NULL CHECK (folder <> ''),
  title        TEXT NOT NULL,
  description  TEXT NOT NULL DEFAULT '',
  UNIQUE (workspace_id, folder)
);
INSERT INTO workspace (id, name, doc_table, view_mode, is_default, created_at) VALUES
  ('default',   'Default',   'doc_default',   'directory', 1, strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
  ('knowledge', 'Knowledge', 'doc_knowledge', 'timeline',  0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'));
"#, doc_table_sql!("doc_default"), doc_table_sql!("doc_knowledge")), LEGACY_ROWS_STEP, DROP_LEGACY_STEP, r#"
ALTER TABLE workspace ADD COLUMN disabled_at TEXT;
ALTER TABLE project ADD COLUMN disabled_at TEXT;
"#, r#"
ALTER TABLE workspace ADD COLUMN icon TEXT;
ALTER TABLE project ADD COLUMN icon TEXT;
"#, r#"
ALTER TABLE workspace ADD COLUMN sort_index INTEGER NOT NULL DEFAULT 0;
ALTER TABLE project ADD COLUMN sort_index INTEGER NOT NULL DEFAULT 0;
UPDATE workspace SET sort_index = (
  SELECT count(*) FROM workspace o
  WHERE o.is_default > workspace.is_default
     OR (o.is_default = workspace.is_default AND (
          lower(o.name) < lower(workspace.name)
          OR (lower(o.name) = lower(workspace.name) AND o.id < workspace.id)))
);
UPDATE project SET sort_index = (
  SELECT count(*) FROM project o
  WHERE o.workspace_id = project.workspace_id
    AND (lower(o.title) < lower(project.title)
         OR (lower(o.title) = lower(project.title) AND o.id < project.id))
);
"#, r#"
ALTER TABLE project ADD COLUMN is_inbox INTEGER NOT NULL DEFAULT 0 CHECK (is_inbox IN (0, 1));
UPDATE project SET is_inbox = 1 WHERE folder = 'inbox';
CREATE UNIQUE INDEX project_one_inbox ON project(workspace_id) WHERE is_inbox = 1;
CREATE TRIGGER project_keys_immutable BEFORE UPDATE OF id, is_inbox ON project
BEGIN SELECT RAISE(ABORT, 'project id and is_inbox cannot change'); END;
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
