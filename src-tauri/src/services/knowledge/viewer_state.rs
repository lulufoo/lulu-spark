//! Knowledge viewer current state: `{cache_dir}/knowledge-viewer-state.json`.
//!
//! File missing or corrupt → empty `{ id }`, no overwrite.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::paths;
use crate::repositories::atomic_json;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct ViewerStateFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    id: String,
}

fn default_version() -> u32 {
    1
}

fn file_path() -> Result<PathBuf, String> {
    paths::knowledge_viewer_state_path().map_err(|e| format!("{e:?}"))
}

fn sanitize(id: &str) -> ViewerStateFile {
    ViewerStateFile {
        version: 1,
        id: id.trim().to_string(),
    }
}

fn save_unlocked(data: &ViewerStateFile) -> Result<(), String> {
    let path = file_path()?;
    let value = json!({
        "version": data.version,
        "id": data.id,
    });
    atomic_json::write_json(&path, &value)
}

fn load_unlocked() -> ViewerStateFile {
    let Ok(path) = file_path() else {
        return ViewerStateFile::default();
    };
    if !path.is_file() {
        return ViewerStateFile::default();
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return ViewerStateFile::default();
    };
    match serde_json::from_str::<ViewerStateFile>(&text) {
        Ok(data) => sanitize(&data.id),
        Err(_) => ViewerStateFile::default(),
    }
}

fn as_json(data: &ViewerStateFile) -> serde_json::Value {
    json!({ "id": data.id })
}

pub fn get_json() -> Result<serde_json::Value, String> {
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    Ok(as_json(&load_unlocked()))
}

pub fn set_json(id: &str) -> Result<serde_json::Value, String> {
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let data = sanitize(id);
    save_unlocked(&data)?;
    Ok(as_json(&data))
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/viewer_state.rs"]
mod tests;
