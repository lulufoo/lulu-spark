//! Read Later queue persisted at `{cache_dir}/read_later.json`.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_entry_id;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadLaterEntry {
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(rename = "saved_at")]
    pub saved_at: String,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadLaterFile {
    pub version: u32,
    pub entries: Vec<ReadLaterEntry>,
}

pub fn read_later_path() -> Result<PathBuf, crate::config::paths::PathsError> {
    Ok(paths::cache_dir()?.join("read_later.json"))
}

fn default_file() -> ReadLaterFile {
    ReadLaterFile {
        version: 1,
        entries: vec![],
    }
}

fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().expect("read_later write lock");
    f()
}

fn load_file_unlocked() -> ReadLaterFile {
    let path = match read_later_path() {
        Ok(p) => p,
        Err(_) => return default_file(),
    };
    if !path.is_file() {
        return default_file();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        let empty = default_file();
        let _ = save_file_unlocked(&empty);
        return empty;
    };
    match serde_json::from_str::<ReadLaterFile>(&text) {
        Ok(file) => file,
        Err(_) => {
            let empty = default_file();
            let _ = save_file_unlocked(&empty);
            empty
        }
    }
}

fn save_file_unlocked(file: &ReadLaterFile) -> Result<(), String> {
    let path = read_later_path().map_err(|e| format!("{e:?}"))?;
    let value = serde_json::to_value(file).map_err(|e| e.to_string())?;
    atomic_json::write_json(&path, &value)
}

fn entry_to_value(entry: &ReadLaterEntry) -> Value {
    serde_json::to_value(entry).unwrap_or_else(|_| json!({}))
}

pub fn create_entry(url: &str, title: Option<&str>) -> Value {
    let url = url.trim();
    if url.is_empty() {
        return json!({ "error": "Missing url", "_status": 400 });
    }
    let title = title.unwrap_or("").trim().to_string();

    with_write_lock(|| {
        let mut file = load_file_unlocked();
        let entry = ReadLaterEntry {
            id: random_entry_id(),
            url: url.to_string(),
            title,
            saved_at: Utc::now().to_rfc3339(),
            read: false,
        };
        file.entries.push(entry.clone());
        match save_file_unlocked(&file) {
            Ok(()) => json!({ "entry": entry_to_value(&entry), "_status": 201 }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn list_entries() -> Value {
    let file = load_file_unlocked();
    let mut entries: Vec<Value> = file.entries.iter().map(entry_to_value).collect();
    entries.sort_by(|a, b| {
        let a_ts = a.get("saved_at").and_then(|v| v.as_str()).unwrap_or("");
        let b_ts = b.get("saved_at").and_then(|v| v.as_str()).unwrap_or("");
        b_ts.cmp(a_ts)
    });
    Value::Array(entries)
}

pub fn mark_read(id: &str, read: bool) -> Value {
    with_write_lock(|| {
        let mut file = load_file_unlocked();
        let Some(idx) = file.entries.iter().position(|e| e.id == id) else {
            return json!({ "error": "Not found", "_status": 404 });
        };
        file.entries[idx].read = read;
        let entry = file.entries[idx].clone();
        match save_file_unlocked(&file) {
            Ok(()) => json!({ "entry": entry_to_value(&entry), "_status": 200 }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn delete_entry(id: &str) -> Value {
    with_write_lock(|| {
        let mut file = load_file_unlocked();
        let Some(idx) = file.entries.iter().position(|e| e.id == id) else {
            return json!({ "error": "Not found", "_status": 404 });
        };
        let removed = file.entries.remove(idx);
        match save_file_unlocked(&file) {
            Ok(()) => json!({ "entry": entry_to_value(&removed), "_status": 200 }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/read_later.rs"]
mod tests;
