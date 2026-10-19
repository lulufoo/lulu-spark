//! Reads and writes of the `document` table: one row per document, `doc_id` the global key.
//! A document's workspace is its project's workspace; the row stores only `project_id`.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;

use super::json_column::{read_array, write_array};

#[derive(Debug, Clone, PartialEq)]
pub struct DocRow {
    pub doc_id: String,
    pub raw_path: String,
    pub digest_path: Option<String>,
    pub source_type: Option<String>,
    pub created_at: Option<String>,
    pub importance: Option<String>,
    pub done: bool,
    pub links: Value,
    pub comments: Value,
    pub tags: Value,
    pub highlights: Value,
    pub translations: Value,
    pub title: Option<String>,
    pub project_id: String,
}

impl DocRow {
    pub fn new(doc_id: &str, raw_path: &str, project_id: &str) -> Self {
        Self {
            doc_id: doc_id.to_string(),
            raw_path: raw_path.to_string(),
            digest_path: None,
            source_type: None,
            created_at: None,
            importance: None,
            done: false,
            links: Value::Array(Vec::new()),
            comments: Value::Array(Vec::new()),
            tags: Value::Array(Vec::new()),
            highlights: Value::Array(Vec::new()),
            translations: Value::Array(Vec::new()),
            title: None,
            project_id: project_id.to_string(),
        }
    }
}

const COLUMNS: &str = "doc_id, raw_path, digest_path, source_type, created_at, importance, done,
                       links, comments, tags, highlights, translations, title, project_id";

fn map_row(row: &Row<'_>) -> rusqlite::Result<DocRow> {
    Ok(DocRow {
        doc_id: row.get(0)?,
        raw_path: row.get(1)?,
        digest_path: row.get(2)?,
        source_type: row.get(3)?,
        created_at: row.get(4)?,
        importance: row.get(5)?,
        done: row.get::<_, i64>(6)? != 0,
        links: read_array(row, 7)?,
        comments: read_array(row, 8)?,
        tags: read_array(row, 9)?,
        highlights: read_array(row, 10)?,
        translations: read_array(row, 11)?,
        title: row.get(12)?,
        project_id: row.get(13)?,
    })
}

pub fn insert_doc(conn: &Connection, doc: &DocRow) -> Result<(), String> {
    conn.execute(
        &format!("INSERT INTO document({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"),
        params![
            doc.doc_id,
            doc.raw_path,
            doc.digest_path,
            doc.source_type,
            doc.created_at,
            doc.importance,
            doc.done as i64,
            write_array(&doc.links)?,
            write_array(&doc.comments)?,
            write_array(&doc.tags)?,
            write_array(&doc.highlights)?,
            write_array(&doc.translations)?,
            doc.title,
            doc.project_id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Replace every column of the row with this `doc_id`. `false` when it does not exist.
pub fn update_doc(conn: &Connection, doc: &DocRow) -> Result<bool, String> {
    let changed = conn
        .execute(
            &format!(
                "UPDATE document SET raw_path = ?2, digest_path = ?3, source_type = ?4, created_at = ?5,
                        importance = ?6, done = ?7, links = ?8, comments = ?9, tags = ?10,
                        highlights = ?11, translations = ?12, title = ?13, project_id = ?14
                 WHERE doc_id = ?1"
            ),
            params![
                doc.doc_id,
                doc.raw_path,
                doc.digest_path,
                doc.source_type,
                doc.created_at,
                doc.importance,
                doc.done as i64,
                write_array(&doc.links)?,
                write_array(&doc.comments)?,
                write_array(&doc.tags)?,
                write_array(&doc.highlights)?,
                write_array(&doc.translations)?,
                doc.title,
                doc.project_id,
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}

pub fn get_doc(conn: &Connection, doc_id: &str) -> Result<Option<DocRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM document WHERE doc_id = ?1"),
        params![doc_id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Every document of the workspace (those whose project belongs to it), ordered by `doc_id`.
pub fn list_docs(conn: &Connection, workspace_id: &str) -> Result<Vec<DocRow>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM document
             WHERE project_id IN (SELECT id FROM project WHERE workspace_id = ?1) ORDER BY doc_id"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![workspace_id], map_row).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// `false` when the row did not exist.
pub fn delete_doc(conn: &Connection, doc_id: &str) -> Result<bool, String> {
    let changed = conn
        .execute(&format!("DELETE FROM document WHERE doc_id = ?1"), params![doc_id])
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}

/// Documents per project (all workspaces; project ids are global) in one query. A project with no document has no entry.
pub fn count_docs_by_project(conn: &Connection) -> Result<HashMap<String, i64>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT project_id, count(*) FROM document GROUP BY project_id"))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<HashMap<_, _>, _>>().map_err(|e| e.to_string())
}

/// How many documents belong to the project.
pub fn count_docs_in_project(conn: &Connection, project_id: &str) -> Result<i64, String> {
    conn.query_row(
        &format!("SELECT count(*) FROM document WHERE project_id = ?1"),
        params![project_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}
