//! Basic reads and writes of one `knowledge_directory` row.

use rusqlite::{params, Connection, OptionalExtension, Row};

pub const UNCATEGORIZED: &str = "Uncategorized";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeDirectoryRow {
    pub id: String,
    pub folder: String,
    pub category: String,
    pub description: String,
}

impl KnowledgeDirectoryRow {
    pub fn new(id: &str, folder: &str, category: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            folder: folder.to_string(),
            category: category.to_string(),
            description: description.to_string(),
        }
    }
}

const COLUMNS: &str = "id, folder, category, description";

fn map_row(row: &Row<'_>) -> rusqlite::Result<KnowledgeDirectoryRow> {
    Ok(KnowledgeDirectoryRow {
        id: row.get(0)?,
        folder: row.get(1)?,
        category: row.get(2)?,
        description: row.get(3)?,
    })
}

pub fn insert_knowledge_directory(
    conn: &Connection,
    row: &KnowledgeDirectoryRow,
) -> Result<(), String> {
    conn.execute(
        &format!("INSERT INTO knowledge_directory({COLUMNS}) VALUES (?1, ?2, ?3, ?4)"),
        params![row.id, row.folder, row.category, row.description],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Replace every column of the row with this `id`. `false` when it does not exist.
pub fn update_knowledge_directory(
    conn: &Connection,
    row: &KnowledgeDirectoryRow,
) -> Result<bool, String> {
    conn.execute(
        "UPDATE knowledge_directory SET folder = ?2, category = ?3, description = ?4 WHERE id = ?1",
        params![row.id, row.folder, row.category, row.description],
    )
    .map(|changed| changed > 0)
    .map_err(|e| e.to_string())
}

pub fn get_knowledge_directory(
    conn: &Connection,
    id: &str,
) -> Result<Option<KnowledgeDirectoryRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM knowledge_directory WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn directory_folder(conn: &Connection, directory_id: &str) -> Result<String, String> {
    get_knowledge_directory(conn, directory_id)?
        .map(|row| row.folder)
        .ok_or_else(|| format!("unknown directory_id: {directory_id}"))
}

pub fn get_knowledge_directory_by_folder(
    conn: &Connection,
    folder: &str,
) -> Result<Option<KnowledgeDirectoryRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM knowledge_directory WHERE folder = ?1"),
        params![folder],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Folder, case-insensitive.
pub fn list_knowledge_directories(conn: &Connection) -> Result<Vec<KnowledgeDirectoryRow>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM knowledge_directory ORDER BY lower(folder)"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Uncategorized first, then other used names (case-insensitive).
pub fn list_knowledge_category_names(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT category FROM knowledge_directory
             WHERE category <> ?1
             ORDER BY lower(category)",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![UNCATEGORIZED], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut names = vec![UNCATEGORIZED.to_string()];
    for name in rows {
        names.push(name.map_err(|e| e.to_string())?);
    }
    Ok(names)
}

pub fn rename_knowledge_category(conn: &Connection, from: &str, to: &str) -> Result<usize, String> {
    conn.execute(
        "UPDATE knowledge_directory SET category = ?2 WHERE category = ?1",
        params![from, to],
    )
    .map_err(|e| e.to_string())
}

pub fn clear_knowledge_category(conn: &Connection, name: &str) -> Result<usize, String> {
    conn.execute(
        "UPDATE knowledge_directory SET category = ?2 WHERE category = ?1",
        params![name, UNCATEGORIZED],
    )
    .map_err(|e| e.to_string())
}

/// `false` when the row did not exist.
pub fn delete_knowledge_directory(conn: &Connection, id: &str) -> Result<bool, String> {
    conn.execute("DELETE FROM knowledge_directory WHERE id = ?1", params![id])
        .map(|changed| changed > 0)
        .map_err(|e| e.to_string())
}
