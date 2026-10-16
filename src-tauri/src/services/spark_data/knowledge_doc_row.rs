//! Basic reads and writes of one `knowledge_doc` row.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;

use super::json_column::{read_array, write_array};
use super::knowledge_directory_row::{
    get_knowledge_directory, get_knowledge_directory_by_folder, insert_knowledge_directory,
    KnowledgeDirectoryRow, UNCATEGORIZED,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeStatus {
    Active,
    Missing,
}

impl KnowledgeStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Missing => "missing",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeDocRow {
    pub doc_id: String,
    pub path: String,
    pub directory_id: String,
    pub status: KnowledgeStatus,
    pub comments: Value,
    pub links: Value,
    pub highlights: Value,
}

impl KnowledgeDocRow {
    pub fn new(doc_id: &str, directory_id: &str, path: &str) -> Self {
        Self {
            doc_id: doc_id.to_string(),
            path: path.to_string(),
            directory_id: directory_id.to_string(),
            status: KnowledgeStatus::Active,
            comments: Value::Array(Vec::new()),
            links: Value::Array(Vec::new()),
            highlights: Value::Array(Vec::new()),
        }
    }
}

const COLUMNS: &str = "doc_id, path, directory_id, status, comments, links, highlights";

fn map_row(row: &Row<'_>) -> rusqlite::Result<KnowledgeDocRow> {
    let status = match row.get::<_, String>(3)?.as_str() {
        "missing" => KnowledgeStatus::Missing,
        _ => KnowledgeStatus::Active,
    };
    Ok(KnowledgeDocRow {
        doc_id: row.get(0)?,
        path: row.get(1)?,
        directory_id: row.get(2)?,
        status,
        comments: read_array(row, 4)?,
        links: read_array(row, 5)?,
        highlights: read_array(row, 6)?,
    })
}

pub fn directory_id_for_folder(conn: &Connection, folder: &str) -> Result<String, String> {
    if let Some(row) = get_knowledge_directory_by_folder(conn, folder)? {
        return Ok(row.id);
    }
    insert_knowledge_directory(
        conn,
        &KnowledgeDirectoryRow::new(folder, folder, UNCATEGORIZED, ""),
    )?;
    Ok(folder.to_string())
}

fn ensure_directory(conn: &Connection, directory_id: &str) -> Result<(), String> {
    if directory_id.is_empty() {
        return Err("directory_id is required".into());
    }
    if get_knowledge_directory(conn, directory_id)?.is_some() {
        return Ok(());
    }
    insert_knowledge_directory(
        conn,
        &KnowledgeDirectoryRow::new(directory_id, directory_id, UNCATEGORIZED, ""),
    )
}

pub fn insert_knowledge_doc(conn: &Connection, doc: &KnowledgeDocRow) -> Result<(), String> {
    ensure_directory(conn, &doc.directory_id)?;
    conn.execute(
        &format!("INSERT INTO knowledge_doc({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
        params![
            doc.doc_id,
            doc.path,
            doc.directory_id,
            doc.status.as_str(),
            write_array(&doc.comments)?,
            write_array(&doc.links)?,
            write_array(&doc.highlights)?,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Replace every column of the row with this `doc_id`. `false` when it does not exist.
pub fn update_knowledge_doc(conn: &Connection, doc: &KnowledgeDocRow) -> Result<bool, String> {
    ensure_directory(conn, &doc.directory_id)?;
    let changed = conn
        .execute(
            "UPDATE knowledge_doc SET path = ?2, directory_id = ?3, status = ?4,
                    comments = ?5, links = ?6, highlights = ?7
             WHERE doc_id = ?1",
            params![
                doc.doc_id,
                doc.path,
                doc.directory_id,
                doc.status.as_str(),
                write_array(&doc.comments)?,
                write_array(&doc.links)?,
                write_array(&doc.highlights)?,
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}

pub fn get_knowledge_doc(conn: &Connection, doc_id: &str) -> Result<Option<KnowledgeDocRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM knowledge_doc WHERE doc_id = ?1"),
        params![doc_id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn find_knowledge_doc_id(
    conn: &Connection,
    directory_id: &str,
    path: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT doc_id FROM knowledge_doc WHERE directory_id = ?1 AND path = ?2",
        params![directory_id, path],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// `false` when the row did not exist.
pub fn delete_knowledge_doc(conn: &Connection, doc_id: &str) -> Result<bool, String> {
    let changed = conn
        .execute("DELETE FROM knowledge_doc WHERE doc_id = ?1", params![doc_id])
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}

/// `(folder, path) → doc_id` of every `active` record; `missing` ones stay out.
pub fn active_knowledge_doc_ids(
    conn: &Connection,
) -> Result<HashMap<(String, String), String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT k.folder, d.path, d.doc_id
             FROM knowledge_doc d
             INNER JOIN knowledge_directory k ON k.id = d.directory_id
             WHERE d.status = 'active'",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(((row.get::<_, String>(0)?, row.get::<_, String>(1)?), row.get::<_, String>(2)?))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<HashMap<_, _>, _>>().map_err(|e| e.to_string())
}

/// `doc_id` of every record whose directory is not `directory_id`.
pub fn knowledge_doc_ids_outside_directory(
    conn: &Connection,
    directory_id: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT doc_id FROM knowledge_doc WHERE directory_id <> ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![directory_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}
