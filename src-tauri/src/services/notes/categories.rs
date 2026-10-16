//! Notes category registry (`note_category` in spark-data).
//! Add/delete changes the allow-list for switch-project and new notes only.

use std::collections::HashSet;
use std::sync::Mutex;

use chrono::{FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use rusqlite::Connection;

use crate::services::id::random_alnum6;
use crate::services::spark_data::{
    self, delete_note_category, get_note_category, insert_note_category, list_note_categories,
    update_note_category, NoteCategoryRow,
};

pub const INBOX_ID: &str = "inbox";
const INBOX_TITLE: &str = "Inbox";
const ID_MAX_LEN: usize = 64;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesCategory {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    /// First path segment for notes. Empty in old files means `id`.
    #[serde(default)]
    pub folder: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotesCatError {
    BadRequest(String),
    NotFound(String),
    Conflict(String),
    Io(String),
}

impl NotesCatError {
    fn status(&self) -> u16 {
        match self {
            Self::BadRequest(_) => 400,
            Self::NotFound(_) => 404,
            Self::Conflict(_) => 409,
            Self::Io(_) => 500,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::BadRequest(m) | Self::NotFound(m) | Self::Conflict(m) | Self::Io(m) => m,
        }
    }

    pub fn into_value(self) -> Value {
        json!({ "error": self.message(), "_status": self.status() })
    }
}

fn inbox_row() -> NoteCategoryRow {
    NoteCategoryRow::new(INBOX_ID, INBOX_ID, INBOX_TITLE, "")
}

fn from_row(row: NoteCategoryRow) -> NotesCategory {
    NotesCategory {
        id: row.id,
        title: row.title,
        description: row.description,
        folder: row.folder,
    }
}

pub fn category_folder(cat: &NotesCategory) -> &str {
    if cat.folder.is_empty() {
        cat.id.as_str()
    } else {
        cat.folder.as_str()
    }
}

pub fn generate_category_id() -> String {
    let tz = FixedOffset::east_opt(8 * 3600).expect("UTC+8");
    let now = Utc::now().with_timezone(&tz);
    format!(
        "{}{:03}{}",
        now.format("%Y%m%d%H%M%S"),
        now.timestamp_subsec_millis(),
        random_alnum6()
    )
}

fn mint_unique_id(used: &HashSet<String>) -> Result<String, NotesCatError> {
    for _ in 0..8 {
        let id = generate_category_id();
        if is_valid_category_id(&id) && !used.contains(&id) {
            return Ok(id);
        }
    }
    Err(NotesCatError::Io("Could not allocate category id".into()))
}

pub fn is_valid_category_id(id: &str) -> bool {
    let raw = id.trim();
    !raw.is_empty()
        && raw.len() <= ID_MAX_LEN
        && !raw.contains("..")
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().expect("notes categories write lock");
    f()
}

fn db_err(e: String) -> NotesCatError {
    NotesCatError::Io(e)
}

fn ensure_inbox(conn: &Connection) -> Result<(), NotesCatError> {
    if get_note_category(conn, INBOX_ID).map_err(db_err)?.is_none() {
        insert_note_category(conn, &inbox_row()).map_err(db_err)?;
    }
    Ok(())
}

fn load_all(conn: &Connection) -> Result<Vec<NotesCategory>, NotesCatError> {
    ensure_inbox(conn)?;
    Ok(list_note_categories(conn)
        .map_err(db_err)?
        .into_iter()
        .map(from_row)
        .collect())
}

fn ensure_unlocked() -> Result<Vec<NotesCategory>, NotesCatError> {
    let conn = spark_data::open().map_err(db_err)?;
    load_all(&conn)
}

pub fn ensure_categories() -> Result<Vec<NotesCategory>, NotesCatError> {
    with_write_lock(ensure_unlocked)
}

pub fn known_ids() -> Result<HashSet<String>, NotesCatError> {
    Ok(ensure_categories()?
        .iter()
        .map(|c| category_folder(c).to_string())
        .collect())
}

pub fn list_notes_categories() -> Result<Value, NotesCatError> {
    let categories = serde_json::to_value(&ensure_categories()?).unwrap_or_else(|_| json!([]));
    Ok(json!({ "categories": categories }))
}

pub fn create_notes_category(id: &str, title: &str, description: &str) -> Result<Value, NotesCatError> {
    let requested = id.trim();
    let title = title.trim();
    let description = description.trim();
    if !requested.is_empty() && !is_valid_category_id(requested) {
        return Err(NotesCatError::BadRequest("Invalid id".into()));
    }
    if title.is_empty() {
        return Err(NotesCatError::BadRequest("Missing title".into()));
    }
    with_write_lock(|| {
        let conn = spark_data::open().map_err(db_err)?;
        let existing = load_all(&conn)?;
        let mut used: HashSet<String> = existing.iter().map(|c| c.id.clone()).collect();
        used.extend(existing.iter().map(|c| category_folder(c).to_string()));
        let id = if requested.is_empty() {
            mint_unique_id(&used)?
        } else if used.contains(requested) {
            return Err(NotesCatError::Conflict(format!("Duplicate id: {requested}")));
        } else {
            requested.to_string()
        };
        let row = NoteCategoryRow::new(&id, &id, title, description);
        insert_note_category(&conn, &row).map_err(db_err)?;
        Ok(json!({ "ok": true, "category": from_row(row) }))
    })
}

pub fn update_notes_category(id: &str, title: &str, description: &str) -> Result<Value, NotesCatError> {
    let id = id.trim();
    let title = title.trim();
    let description = description.trim();
    if id.is_empty() {
        return Err(NotesCatError::BadRequest("Missing id".into()));
    }
    if title.is_empty() {
        return Err(NotesCatError::BadRequest("Missing title".into()));
    }
    with_write_lock(|| {
        let conn = spark_data::open().map_err(db_err)?;
        ensure_inbox(&conn)?;
        let Some(mut row) = get_note_category(&conn, id).map_err(db_err)? else {
            return Err(NotesCatError::NotFound("Category not found".into()));
        };
        row.title = title.to_string();
        row.description = description.to_string();
        if !update_note_category(&conn, &row).map_err(db_err)? {
            return Err(NotesCatError::NotFound("Category not found".into()));
        }
        Ok(json!({ "ok": true, "category": from_row(row) }))
    })
}

pub fn delete_notes_category(id: &str) -> Result<Value, NotesCatError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(NotesCatError::BadRequest("Missing id".into()));
    }
    if id == INBOX_ID {
        return Err(NotesCatError::BadRequest("Cannot delete inbox".into()));
    }
    with_write_lock(|| {
        let conn = spark_data::open().map_err(db_err)?;
        ensure_inbox(&conn)?;
        if !delete_note_category(&conn, id).map_err(db_err)? {
            return Err(NotesCatError::NotFound("Category not found".into()));
        }
        Ok(json!({ "ok": true }))
    })
}

pub fn list_notes_categories_value() -> Value {
    list_notes_categories().unwrap_or_else(NotesCatError::into_value)
}

pub fn create_notes_category_value(id: &str, title: &str, description: &str) -> Value {
    create_notes_category(id, title, description).unwrap_or_else(NotesCatError::into_value)
}

pub fn update_notes_category_value(id: &str, title: &str, description: &str) -> Value {
    update_notes_category(id, title, description).unwrap_or_else(NotesCatError::into_value)
}

pub fn delete_notes_category_value(id: &str) -> Value {
    delete_notes_category(id).unwrap_or_else(NotesCatError::into_value)
}

#[cfg(test)]
#[path = "../../unit-tests/services/notes_categories.rs"]
mod tests;
