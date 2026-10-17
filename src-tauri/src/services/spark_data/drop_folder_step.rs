//! Migration step: rebuild `project` without `folder`.
//!
//! `UNIQUE (workspace_id, folder)` blocks `DROP COLUMN`. Document tables keep
//! `REFERENCES project(id)`, so this step runs with foreign keys off (they cannot
//! change inside a transaction) and checks them after the rebuild.

use rusqlite::{OptionalExtension, Connection};

/// Rebuilds `project` without `folder`, then restores the inbox unique index and
/// the immutable-keys trigger that lived on the old table.
pub(super) const DROP_PROJECT_FOLDER_STEP: &str = r#"
CREATE TABLE project_new (
  id           TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspace(id),
  title        TEXT NOT NULL,
  description  TEXT NOT NULL DEFAULT '',
  disabled_at  TEXT,
  icon         TEXT,
  sort_index   INTEGER NOT NULL DEFAULT 0,
  is_inbox     INTEGER NOT NULL DEFAULT 0 CHECK (is_inbox IN (0, 1))
);
INSERT INTO project_new (id, workspace_id, title, description, disabled_at, icon, sort_index, is_inbox)
SELECT id, workspace_id, title, description, disabled_at, icon, sort_index, is_inbox FROM project;
DROP TABLE project;
ALTER TABLE project_new RENAME TO project;
CREATE UNIQUE INDEX project_one_inbox ON project(workspace_id) WHERE is_inbox = 1;
CREATE TRIGGER project_keys_immutable BEFORE UPDATE OF id, is_inbox ON project
BEGIN SELECT RAISE(ABORT, 'project id and is_inbox cannot change'); END;
"#;

/// Run `body` with foreign keys off, then turn them back on and check.
pub(super) fn run_with_foreign_keys_off(
    conn: &Connection,
    body: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    conn.pragma_update(None, "foreign_keys", false)
        .map_err(|e| e.to_string())?;
    let result = body();
    let restore = conn
        .pragma_update(None, "foreign_keys", true)
        .map_err(|e| e.to_string());
    result?;
    restore?;
    let table = conn
        .query_row("PRAGMA foreign_key_check", [], |row| row.get::<_, String>(0))
        .optional()
        .map_err(|e| e.to_string())?;
    match table {
        Some(table) => Err(format!("foreign key check failed on {table}")),
        None => Ok(()),
    }
}
