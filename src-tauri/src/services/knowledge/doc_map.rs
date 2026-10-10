//! Global Knowledge document-id map: `{cache_dir}/knowledge-doc-map.json`.
//!
//! IDs are issued only when a search hit is remembered (32 hex, like notes). The same
//! absolute path reuses the same id; ids issued earlier keep their original length. The file survives process restart. Wiping `cache_dir` voids the ids.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_entry_id;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct MapFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    entries: Vec<MapEntry>,
}

fn default_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MapEntry {
    id: String,
    path: String,
}

fn map_path() -> Result<PathBuf, String> {
    paths::knowledge_doc_map_path().map_err(|e| format!("{e:?}"))
}

fn load_unlocked() -> Result<MapFile, String> {
    let path = map_path()?;
    if !path.is_file() {
        return Ok(MapFile {
            version: 1,
            entries: Vec::new(),
        });
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn save_unlocked(data: &MapFile) -> Result<(), String> {
    let path = map_path()?;
    let value = json!({
        "version": data.version,
        "entries": data.entries,
    });
    atomic_json::write_json(&path, &value)
}

fn normalize_abs(path: &Path) -> Result<String, String> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    Ok(canon.to_string_lossy().into_owned())
}

/// Remember an absolute path. Reuses the existing id when the path is already mapped.
pub fn remember_path(path: &Path) -> Result<String, String> {
    let abs = normalize_abs(path)?;
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let mut data = load_unlocked()?;
    if let Some(entry) = data.entries.iter().find(|e| e.path == abs) {
        return Ok(entry.id.clone());
    }
    let id = random_entry_id();
    data.version = 1;
    data.entries.push(MapEntry {
        id: id.clone(),
        path: abs,
    });
    save_unlocked(&data)?;
    Ok(id)
}

/// Look up the absolute path for a search-issued document id.
pub fn lookup_path(id: &str) -> Result<Option<PathBuf>, String> {
    let id = id.trim();
    if id.is_empty() {
        return Ok(None);
    }
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let data = load_unlocked()?;
    Ok(data
        .entries
        .iter()
        .find(|e| e.id == id)
        .map(|e| PathBuf::from(&e.path)))
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/doc_map.rs"]
mod tests;
