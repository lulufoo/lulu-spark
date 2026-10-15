//! Basic reads and writes of one `note` row.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;

use super::json_column::{read_array, write_array};

#[derive(Debug, Clone, PartialEq)]
pub struct NoteRow {
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
}

impl NoteRow {
    pub fn new(doc_id: &str, raw_path: &str) -> Self {
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
        }
    }
}

const COLUMNS: &str = "doc_id, raw_path, digest_path, source_type, created_at, importance, done,
                       links, comments, tags, highlights, translations, title";

fn map_row(row: &Row<'_>) -> rusqlite::Result<NoteRow> {
    Ok(NoteRow {
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
    })
}

pub fn insert_note(conn: &Connection, note: &NoteRow) -> Result<(), String> {
    conn.execute(
        &format!("INSERT INTO note({COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"),
        params![
            note.doc_id,
            note.raw_path,
            note.digest_path,
            note.source_type,
            note.created_at,
            note.importance,
            note.done as i64,
            write_array(&note.links)?,
            write_array(&note.comments)?,
            write_array(&note.tags)?,
            write_array(&note.highlights)?,
            write_array(&note.translations)?,
            note.title,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Replace every column of the row with this `doc_id`. `false` when it does not exist.
pub fn update_note(conn: &Connection, note: &NoteRow) -> Result<bool, String> {
    let changed = conn
        .execute(
            "UPDATE note SET raw_path = ?2, digest_path = ?3, source_type = ?4, created_at = ?5,
                    importance = ?6, done = ?7, links = ?8, comments = ?9, tags = ?10,
                    highlights = ?11, translations = ?12, title = ?13
             WHERE doc_id = ?1",
            params![
                note.doc_id,
                note.raw_path,
                note.digest_path,
                note.source_type,
                note.created_at,
                note.importance,
                note.done as i64,
                write_array(&note.links)?,
                write_array(&note.comments)?,
                write_array(&note.tags)?,
                write_array(&note.highlights)?,
                write_array(&note.translations)?,
                note.title,
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}

pub fn get_note(conn: &Connection, doc_id: &str) -> Result<Option<NoteRow>, String> {
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM note WHERE doc_id = ?1"),
        params![doc_id],
        map_row,
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// `raw_path` is not unique (the real `index.json` has two ids sharing one path), so
/// this returns every match.
pub fn find_note_ids_by_path(conn: &Connection, raw_path: &str) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT doc_id FROM note WHERE raw_path = ?1 ORDER BY doc_id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![raw_path], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// `false` when the row did not exist.
pub fn delete_note(conn: &Connection, doc_id: &str) -> Result<bool, String> {
    let changed = conn
        .execute("DELETE FROM note WHERE doc_id = ?1", params![doc_id])
        .map_err(|e| e.to_string())?;
    Ok(changed > 0)
}
