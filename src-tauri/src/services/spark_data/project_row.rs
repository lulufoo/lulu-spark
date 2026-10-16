//! Basic reads and writes of one `project` row (one global table, keyed by workspace).

use rusqlite::{params, Connection, OptionalExtension, Row};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub id: String,
    pub workspace_id: String,
    pub folder: String,
    pub title: String,
    pub description: String,
}

impl ProjectRow {
    pub fn new(id: &str, workspace_id: &str, folder: &str, title: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            folder: folder.to_string(),
            title: title.to_string(),
            description: description.to_string(),
        }
    }
}

const COLUMNS: &str = "id, workspace_id, folder, title, description";

fn map_row(row: &Row<'_>) -> rusqlite::Result<ProjectRow> {
    Ok(ProjectRow {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        folder: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
    })
}

pub fn insert_project(conn: &Connection, row: &ProjectRow) -> Result<(), String> {
    conn.execute(
        &format!("INSERT INTO project({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5)"),
        params![row.id, row.workspace_id, row.folder, row.title, row.description],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Replace `folder`, `title` and `description` of the row with this `id`.
/// A project never moves between workspaces. `false` when it does not exist.
pub fn update_project(conn: &Connection, row: &ProjectRow) -> Result<bool, String> {
    conn.execute(
        "UPDATE project SET folder = ?2, title = ?3, description = ?4 WHERE id = ?1",
        params![row.id, row.folder, row.title, row.description],
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

pub fn get_project_by_folder(
    conn: &Connection,
    workspace_id: &str,
    folder: &str,
) -> Result<Option<ProjectRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM project WHERE workspace_id = ?1 AND folder = ?2"),
        params![workspace_id, folder],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Projects of one workspace, by title (case-insensitive).
pub fn list_projects(conn: &Connection, workspace_id: &str) -> Result<Vec<ProjectRow>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM project WHERE workspace_id = ?1 ORDER BY lower(title)"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![workspace_id], map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// `false` when the row did not exist. Fails while documents still reference it.
pub fn delete_project(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.execute("DELETE FROM project WHERE id = ?1", params![id])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}
