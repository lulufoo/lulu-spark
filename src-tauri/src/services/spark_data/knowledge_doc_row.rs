//! Basic reads and writes of one `knowledge_doc` row.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;

use super::json_column::{read_array, write_array};

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
    pub repo: String,
    pub path: String,
    pub status: KnowledgeStatus,
    pub comments: Value,
    pub links: Value,
    pub highlights: Value,
}

impl KnowledgeDocRow {
    pub fn new(doc_id: &str, repo: &str, path: &str) -> Self {
        Self {
            doc_id: doc_id.to_string(),
            repo: repo.to_string(),
            path: path.to_string(),
            status: KnowledgeStatus::Active,
            comments: Value::Array(Vec::new()),
            links: Value::Array(Vec::new()),
            highlights: Value::Array(Vec::new()),
        }
    }
}

const COLUMNS: &str = "doc_id, repo, path, status, comments, links, highlights";

fn map_row(row: &Row<'_>) -> rusqlite::Result<KnowledgeDocRow> {
    let status = match row.get::<_, String>(3)?.as_str() {
        "missing" => KnowledgeStatus::Missing,
        _ => KnowledgeStatus::Active,
    };
    Ok(KnowledgeDocRow {
        doc_id: row.get(0)?,
        repo: row.get(1)?,
        path: row.get(2)?,
        status,
        comments: read_array(row, 4)?,
        links: read_array(row, 5)?,
        highlights: read_array(row, 6)?,
    })
}

pub fn insert_knowledge_doc(conn: &Connection, doc: &KnowledgeDocRow) -> Result<(), String> {
    conn.execute(
        &format!("INSERT INTO knowledge_doc({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"),
        params![
            doc.doc_id,
            doc.repo,
            doc.path,
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
    let changed = conn
        .execute(
            "UPDATE knowledge_doc SET repo = ?2, path = ?3, status = ?4, comments = ?5,
                    links = ?6, highlights = ?7
             WHERE doc_id = ?1",
            params![
                doc.doc_id,
                doc.repo,
                doc.path,
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
    repo: &str,
    path: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT doc_id FROM knowledge_doc WHERE repo = ?1 AND path = ?2",
        params![repo, path],
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

/// `(repo, path) → doc_id` of every `active` record; `missing` ones stay out.
pub fn active_knowledge_doc_ids(
    conn: &Connection,
) -> Result<HashMap<(String, String), String>, String> {
    let mut stmt = conn
        .prepare("SELECT repo, path, doc_id FROM knowledge_doc WHERE status = 'active'")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(((row.get::<_, String>(0)?, row.get::<_, String>(1)?), row.get::<_, String>(2)?))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<HashMap<_, _>, _>>().map_err(|e| e.to_string())
}

/// `doc_id` of every record whose repo is not `repo` (the clone's directory name).
pub fn knowledge_doc_ids_outside_repo(
    conn: &Connection,
    repo: &str,
) -> Result<std::collections::HashSet<String>, String> {
    let mut stmt = conn
        .prepare("SELECT doc_id FROM knowledge_doc WHERE repo <> ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![repo], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
}
