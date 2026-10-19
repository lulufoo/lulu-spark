//! `spark-data.sqlite`: durable metadata (workspaces, projects, documents, tags).
//!
//! No process-wide connection: like `search_index` and the agent session store,
//! every operation calls [`open`] and gets its own `Connection`. Writes are
//! serialized by SQLite itself (WAL allows a single writer; `busy_timeout` makes
//! a second writer wait instead of failing).
//!
//! The schema version lives in `PRAGMA user_version`. An empty file is created
//! straight at [`BASELINE_VERSION`] from one whole DDL; there is no upgrade chain
//! below it, and a file at a lower version is refused. Changes after the baseline
//! are steps in [`MIGRATIONS`]: `MIGRATIONS[i]` takes a file from
//! `BASELINE_VERSION + i` to `BASELINE_VERSION + i + 1`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

use super::workspace_row::create_default_workspace;
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

/// The version a new file is created at, and the lowest version this build opens.
pub(crate) const BASELINE_VERSION: usize = 17;

/// Steps after the baseline. Append a step to change the schema; never edit an
/// existing one. Empty while the baseline is the latest version.
pub(crate) const MIGRATIONS: [&str; 0] = [];

/// Whole schema at [`BASELINE_VERSION`]. The `Default` workspace, its document
/// table and its inbox are added by [`ensure_default_workspace`] because their id is random.
const BASELINE_SQL: &str = r#"
CREATE TABLE tag (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
CREATE TABLE workspace (
  id         TEXT PRIMARY KEY CHECK (id <> '' AND id NOT GLOB '*[^a-z0-9_]*'),
  name       TEXT NOT NULL UNIQUE CHECK (name <> ''),
  doc_table  TEXT NOT NULL UNIQUE CHECK (doc_table = 'doc_' || id),
  view_mode  TEXT NOT NULL DEFAULT 'directory' CHECK (view_mode IN ('directory', 'timeline')),
  is_default INTEGER NOT NULL DEFAULT 0 CHECK (is_default IN (0, 1)),
  created_at TEXT,
  disabled_at TEXT,
  icon TEXT,
  sort_index INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX workspace_one_default ON workspace(is_default) WHERE is_default = 1;
CREATE TRIGGER workspace_keys_immutable BEFORE UPDATE OF id, doc_table, is_default ON workspace
BEGIN SELECT RAISE(ABORT, 'workspace id, doc_table and is_default cannot change'); END;
CREATE TRIGGER workspace_default_undeletable BEFORE DELETE ON workspace WHEN OLD.is_default = 1
BEGIN SELECT RAISE(ABORT, 'the default workspace cannot be deleted'); END;
CREATE TABLE project (
  id           TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspace(id),
  name         TEXT NOT NULL,
  description  TEXT NOT NULL DEFAULT '',
  disabled_at  TEXT,
  icon         TEXT,
  sort_index   INTEGER NOT NULL DEFAULT 0,
  is_inbox     INTEGER NOT NULL DEFAULT 0 CHECK (is_inbox IN (0, 1))
);
CREATE UNIQUE INDEX project_one_inbox ON project(workspace_id) WHERE is_inbox = 1;
CREATE TRIGGER project_keys_immutable BEFORE UPDATE OF id, is_inbox ON project
BEGIN SELECT RAISE(ABORT, 'project id and is_inbox cannot change'); END;
"#;

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
    // Refuse before any pragma: switching to WAL rewrites the file header, and a file this
    // build does not open must stay as it was.
    check_supported(user_version(&conn)?, BASELINE_VERSION + steps.len())?;
    enable_wal(&conn)?;
    conn.execute_batch("PRAGMA foreign_keys=ON;").map_err(|e| e.to_string())?;
    migrate(&conn, steps)?;
    Ok(conn)
}

/// Switch the file to WAL. When several connections switch a new file at once, SQLite answers
/// "locked" at once instead of waiting on `busy_timeout`, so retry for as long as a writer would wait.
fn enable_wal(conn: &Connection) -> Result<(), String> {
    let deadline = std::time::Instant::now() + BUSY_TIMEOUT;
    loop {
        match conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(())) {
            Ok(()) => return Ok(()),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::DatabaseBusy && std::time::Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn user_version(conn: &Connection) -> Result<usize, String> {
    conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .map(|v| v as usize)
        .map_err(|e| e.to_string())
}

fn set_user_version(conn: &Connection, version: usize) -> Result<(), String> {
    conn.pragma_update(None, "user_version", version)
        .map_err(|e| e.to_string())
}

/// Run `body` inside `BEGIN IMMEDIATE`: the write lock is held from the start, so the
/// version `body` reads is the version nobody else can change until it commits.
fn with_write_lock(conn: &Connection, body: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| e.to_string())?;
    match body() {
        Ok(()) => conn.execute_batch("COMMIT").map_err(|e| e.to_string()),
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn default_workspace_exists(conn: &Connection) -> Result<bool, String> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM workspace WHERE is_default = 1)", [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

/// Create the `Default` workspace (the same creation as any workspace: row, document table,
/// inbox project, one transaction) when the file has none, and do nothing when it does. Decided from the rows, not from the
/// version, and re-checked under the write lock so concurrent opens seed it once.
fn ensure_default_workspace(conn: &Connection) -> Result<(), String> {
    if default_workspace_exists(conn)? {
        return Ok(());
    }
    with_write_lock(conn, || {
        if default_workspace_exists(conn)? {
            return Ok(());
        }
        create_default_workspace(conn).map(drop)
    })
}

/// A version this build can open: 0 (empty file), or `BASELINE_VERSION..=latest`.
fn check_supported(version: usize, latest: usize) -> Result<(), String> {
    if version != 0 && version < BASELINE_VERSION {
        return Err(format!(
            "{DB_FILE} is at schema version {version}; this build opens version {BASELINE_VERSION} and later, \
             earlier versions are not supported"
        ));
    }
    if version > latest {
        return Err(format!(
            "{DB_FILE} is at schema version {version}, newer than this build ({latest})"
        ));
    }
    Ok(())
}

/// Bring the file to the latest version, then make sure a `Default` workspace exists.
/// Every write re-reads its precondition after taking the write lock, so connections that
/// open at the same moment never repeat a step. A file below the baseline, or newer than
/// this build, is refused untouched.
pub(crate) fn migrate(conn: &Connection, steps: &[&str]) -> Result<(), String> {
    let latest = BASELINE_VERSION + steps.len();
    let version = user_version(conn)?;
    check_supported(version, latest)?;
    if version != latest {
        if version == 0 {
            with_write_lock(conn, || {
                if user_version(conn)? != 0 {
                    return Ok(());
                }
                conn.execute_batch(BASELINE_SQL).map_err(|e| e.to_string())?;
                set_user_version(conn, BASELINE_VERSION)
            })?;
        }
        for (i, step) in steps.iter().enumerate() {
            let from = BASELINE_VERSION + i;
            with_write_lock(conn, || {
                if user_version(conn)? != from {
                    return Ok(());
                }
                conn.execute_batch(step).map_err(|e| e.to_string())?;
                set_user_version(conn, from + 1)
            })?;
        }
    }
    ensure_default_workspace(conn)
}
