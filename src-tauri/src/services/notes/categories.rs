//! Notes category registry (`notes/categories.json`).
//! Add/delete changes the allow-list for switch-project and new notes only.

use std::collections::HashSet;
use std::fs;
use std::sync::Mutex;

use chrono::{FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_alnum6;

pub const INBOX_ID: &str = "inbox";
const INBOX_TITLE: &str = "Inbox";
const CATEGORIES_VERSION: u32 = 1;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesCategoriesFile {
    pub version: u32,
    pub categories: Vec<NotesCategory>,
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

fn inbox_category() -> NotesCategory {
    NotesCategory {
        id: INBOX_ID.to_string(),
        title: INBOX_TITLE.to_string(),
        description: String::new(),
        folder: INBOX_ID.to_string(),
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

fn default_file() -> NotesCategoriesFile {
    NotesCategoriesFile {
        version: CATEGORIES_VERSION,
        categories: vec![inbox_category()],
    }
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

fn registry_path() -> Result<std::path::PathBuf, NotesCatError> {
    paths::notes_categories_path().map_err(|e| NotesCatError::Io(format!("{e:?}")))
}

fn load_file_unlocked() -> Result<NotesCategoriesFile, NotesCatError> {
    let path = registry_path()?;
    if !path.is_file() {
        return Ok(default_file());
    }
    let text = fs::read_to_string(&path).map_err(|e| NotesCatError::Io(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| NotesCatError::Io(e.to_string()))
}

fn save_file_unlocked(data: &NotesCategoriesFile) -> Result<(), NotesCatError> {
    let path = registry_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| NotesCatError::Io(e.to_string()))?;
    }
    let value = serde_json::to_value(data).map_err(|e| NotesCatError::Io(e.to_string()))?;
    atomic_json::write_json(&path, &value).map_err(NotesCatError::Io)
}

fn ensure_inbox(file: &mut NotesCategoriesFile) -> bool {
    if file.categories.iter().any(|c| c.id == INBOX_ID) {
        return false;
    }
    file.categories.insert(0, inbox_category());
    true
}

fn sort_categories(file: &mut NotesCategoriesFile) {
    file.categories.sort_by(|a, b| match (a.id.as_str(), b.id.as_str()) {
        (INBOX_ID, _) => std::cmp::Ordering::Less,
        (_, INBOX_ID) => std::cmp::Ordering::Greater,
        _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
    });
}

fn ensure_unlocked() -> Result<NotesCategoriesFile, NotesCatError> {
    let mut file = load_file_unlocked()?;
    let dirty = ensure_inbox(&mut file);
    if dirty || !registry_path()?.is_file() {
        sort_categories(&mut file);
        save_file_unlocked(&file)?;
    }
    Ok(file)
}

pub fn ensure_categories() -> Result<NotesCategoriesFile, NotesCatError> {
    with_write_lock(ensure_unlocked)
}

pub fn known_ids() -> Result<HashSet<String>, NotesCatError> {
    Ok(ensure_categories()?
        .categories
        .iter()
        .map(|c| category_folder(c).to_string())
        .collect())
}

pub fn list_notes_categories() -> Result<Value, NotesCatError> {
    let categories =
        serde_json::to_value(&ensure_categories()?.categories).unwrap_or_else(|_| json!([]));
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
        let mut file = ensure_unlocked()?;
        let used: HashSet<String> = file.categories.iter().map(|c| c.id.clone()).collect();
        let id = if requested.is_empty() {
            mint_unique_id(&used)?
        } else if used.contains(requested) {
            return Err(NotesCatError::Conflict(format!("Duplicate id: {requested}")));
        } else {
            requested.to_string()
        };
        let category = NotesCategory {
            id: id.clone(),
            title: title.to_string(),
            description: description.to_string(),
            folder: id,
        };
        file.categories.push(category.clone());
        sort_categories(&mut file);
        save_file_unlocked(&file)?;
        Ok(json!({ "ok": true, "category": category }))
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
        let mut file = ensure_unlocked()?;
        let Some(cat) = file.categories.iter_mut().find(|c| c.id == id) else {
            return Err(NotesCatError::NotFound("Category not found".into()));
        };
        cat.title = title.to_string();
        cat.description = description.to_string();
        let category = cat.clone();
        save_file_unlocked(&file)?;
        Ok(json!({ "ok": true, "category": category }))
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
        let mut file = ensure_unlocked()?;
        let before = file.categories.len();
        file.categories.retain(|c| c.id != id);
        if file.categories.len() == before {
            return Err(NotesCatError::NotFound("Category not found".into()));
        }
        save_file_unlocked(&file)?;
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
