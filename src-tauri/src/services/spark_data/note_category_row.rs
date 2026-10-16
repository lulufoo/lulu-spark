//! Basic reads and writes of one `note_category` row.

use rusqlite::{params, Connection, OptionalExtension, Row};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteCategoryRow {
    pub id: String,
    pub folder: String,
    pub title: String,
    pub description: String,
}

impl NoteCategoryRow {
    pub fn new(id: &str, folder: &str, title: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            folder: folder.to_string(),
            title: title.to_string(),
            description: description.to_string(),
        }
    }
}

const COLUMNS: &str = "id, folder, title, description";

fn map_row(row: &Row<'_>) -> rusqlite::Result<NoteCategoryRow> {
    Ok(NoteCategoryRow {
        id: row.get(0)?,
        folder: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
    })
}

pub fn insert_note_category(conn: &Connection, row: &NoteCategoryRow) -> Result<(), String> {
    conn.execute(
        &format!("INSERT INTO note_category({COLUMNS}) VALUES (?1, ?2, ?3, ?4)"),
        params![row.id, row.folder, row.title, row.description],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Replace every column of the row with this `id`. `false` when it does not exist.
pub fn update_note_category(conn: &Connection, row: &NoteCategoryRow) -> Result<bool, String> {
    conn.execute(
        "UPDATE note_category SET folder = ?2, title = ?3, description = ?4 WHERE id = ?1",
        params![row.id, row.folder, row.title, row.description],
    )
    .map(|changed| changed > 0)
    .map_err(|e| e.to_string())
}

pub fn get_note_category(conn: &Connection, id: &str) -> Result<Option<NoteCategoryRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM note_category WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn get_note_category_by_folder(
    conn: &Connection,
    folder: &str,
) -> Result<Option<NoteCategoryRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM note_category WHERE folder = ?1"),
        params![folder],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Inbox first, then title (case-insensitive).
pub fn list_note_categories(conn: &Connection) -> Result<Vec<NoteCategoryRow>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM note_category
             ORDER BY CASE WHEN id = 'inbox' THEN 0 ELSE 1 END, lower(title)"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// `false` when the row did not exist.
pub fn delete_note_category(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.execute("DELETE FROM note_category WHERE id = ?1", params![id])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}
