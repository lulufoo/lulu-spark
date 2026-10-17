//! The document each workspace's page had open:
//! `{cache_dir}/workspace-viewer-state/{workspace_id}.json`.
//!
//! File missing or corrupt → empty `{ id }`, no overwrite. A missing file is not seeded.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::spark_data::validate_workspace_id;

use super::resolve::{internal, open_data, resolve_workspace, text_field};

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

fn file_path(workspace_id: &str) -> Result<PathBuf, String> {
    validate_workspace_id(workspace_id)?;
    paths::workspace_viewer_state_path(workspace_id).map_err(|e| format!("{e:?}"))
}

fn load_unlocked(workspace_id: &str) -> ViewerStateFile {
    let Ok(path) = file_path(workspace_id) else {
        return ViewerStateFile::default();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return ViewerStateFile::default();
    };
    match serde_json::from_str::<ViewerStateFile>(&text) {
        Ok(data) => ViewerStateFile {
            version: 1,
            id: data.id.trim().to_string(),
        },
        Err(_) => ViewerStateFile::default(),
    }
}

fn workspace_id_of(payload: &Value) -> Result<String, Value> {
    let conn = open_data()?;
    resolve_workspace(&conn, text_field(payload, "workspace")).map(|ws| ws.id)
}

/// `{ workspace? }` → `{ id }`; `id` is empty when nothing is remembered.
pub fn get_viewer_state(payload: &Value) -> Value {
    let workspace = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let _guard = WRITE_LOCK.lock().ok();
    json!({ "id": load_unlocked(&workspace).id })
}

/// `{ workspace?, id }` → `{ id }`. An empty `id` forgets the document.
pub fn set_viewer_state(payload: &Value) -> Value {
    let workspace = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let id = text_field(payload, "id").unwrap_or("").to_string();
    let _guard = WRITE_LOCK.lock().ok();
    let path = match file_path(&workspace) {
        Ok(path) => path,
        Err(e) => return internal(e),
    };
    let value = json!({ "version": 1, "id": id });
    match atomic_json::write_json(&path, &value) {
        Ok(()) => json!({ "id": id }),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_viewer_state.rs"]
mod tests;
