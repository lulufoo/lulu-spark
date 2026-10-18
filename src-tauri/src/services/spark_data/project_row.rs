//! Basic reads and writes of one `project` row (one global table, keyed by workspace).

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::workspace_row::ensure_same_set;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub description: String,
    /// `None` = active; a timestamp = disabled (hidden from lists and search, data kept).
    pub disabled_at: Option<String>,
    /// Id of a built-in icon; `None` = the default icon. The frontend owns the icon list.
    pub icon: Option<String>,
    /// Position in the user's order inside the workspace; ignored on insert (new rows go last).
    pub sort_index: i64,
    /// The built-in inbox of this workspace. At most one row per workspace.
    pub is_inbox: bool,
}

impl ProjectRow {
    pub fn new(id: &str, workspace_id: &str, name: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            disabled_at: None,
            icon: None,
            sort_index: 0,
            is_inbox: false,
        }
    }
}

const COLUMNS: &str = "id, workspace_id, name, description, disabled_at, icon, sort_index, is_inbox";

fn map_row(row: &Row<'_>) -> rusqlite::Result<ProjectRow> {
    Ok(ProjectRow {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        description: row.get(3)?,
        disabled_at: row.get(4)?,
        icon: row.get(5)?,
        sort_index: row.get(6)?,
        is_inbox: row.get::<_, i64>(7)? != 0,
    })
}

pub fn insert_project(conn: &Connection, row: &ProjectRow) -> Result<(), String> {
    conn.execute(
        "INSERT INTO project(id, workspace_id, name, description, icon, sort_index, is_inbox)
         VALUES (?1, ?2, ?3, ?4, ?5,
                 (SELECT coalesce(max(sort_index) + 1, 0) FROM project WHERE workspace_id = ?2),
                 ?6)",
        params![
            row.id,
            row.workspace_id,
            row.name,
            row.description,
            row.icon,
            if row.is_inbox { 1 } else { 0 }
        ],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Replace `name`, `description` and `icon` of the row with this `id`.
/// A project never moves between workspaces. `false` when it does not exist.
pub fn update_project(conn: &Connection, row: &ProjectRow) -> Result<bool, String> {
    conn.execute(
        "UPDATE project SET name = ?2, description = ?3, icon = ?4 WHERE id = ?1",
        params![row.id, row.name, row.description, row.icon],
    )
    .map(|changed| changed > 0)
    .map_err(|e| e.to_string())
}

pub fn get_project(conn: &Connection, id: &str) -> Result<Option<ProjectRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM project WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// The built-in inbox of this workspace, if the row exists.
pub fn get_inbox_project(conn: &Connection, workspace_id: &str) -> Result<Option<ProjectRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM project WHERE workspace_id = ?1 AND is_inbox = 1"),
        params![workspace_id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn list_where(conn: &Connection, workspace_id: &str, condition: &str) -> Result<Vec<ProjectRow>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM project WHERE workspace_id = ?1 AND {condition} ORDER BY sort_index, id"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![workspace_id], map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Active projects of one workspace in the user's order (`sort_index`).
pub fn list_projects(conn: &Connection, workspace_id: &str) -> Result<Vec<ProjectRow>, String> {
    list_where(conn, workspace_id, "disabled_at IS NULL")
}

/// Disabled projects of one workspace. Only the Settings "Disabled" section reads them.
pub fn list_disabled_projects(conn: &Connection, workspace_id: &str) -> Result<Vec<ProjectRow>, String> {
    list_where(conn, workspace_id, "disabled_at IS NOT NULL")
}

/// Every project of one workspace, disabled ones included.
pub fn list_all_projects(conn: &Connection, workspace_id: &str) -> Result<Vec<ProjectRow>, String> {
    list_where(conn, workspace_id, "1 = 1")
}

/// Disable a project: it disappears from lists, pickers and search, its documents stay and
/// stay readable by id. Disabling twice keeps the first timestamp. `false` when it does not exist.
pub fn disable_project(conn: &Connection, id: &str) -> Result<bool, String> {
    let exists = get_project(conn, id)?.is_some();
    conn.execute(
        "UPDATE project SET disabled_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
         WHERE id = ?1 AND disabled_at IS NULL",
        params![id],
    )
    .map_err(|e| e.to_string())?;
    Ok(exists)
}

/// Enable a disabled project again. `false` when it does not exist.
pub fn enable_project(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.execute("UPDATE project SET disabled_at = NULL WHERE id = ?1", params![id])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}

/// Put every project of one workspace in the order of `ids`. `ids` must name every
/// project of the workspace (disabled ones included) exactly once; anything else is refused
/// and nothing changes.
pub fn reorder_projects(conn: &Connection, workspace_id: &str, ids: &[String]) -> Result<(), String> {
    let known: Vec<String> = list_all_projects(conn, workspace_id)?
        .into_iter()
        .map(|p| p.id)
        .collect();
    ensure_same_set(&known, ids, "project")?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE project SET sort_index = ?2 WHERE id = ?1",
            params![id, index as i64],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}
