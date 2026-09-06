//! Knowledge viewer current state: `{cache_dir}/knowledge-viewer-state.json`.
//!
//! File missing or corrupt → empty `{ repo, path }`, no overwrite.

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
    repo: String,
    #[serde(default)]
    path: String,
}

fn default_version() -> u32 {
    1
}

fn file_path() -> Result<PathBuf, String> {
    paths::knowledge_viewer_state_path().map_err(|e| format!("{e:?}"))
}

fn is_md_path(path: &str) -> bool {
    path.trim().to_ascii_lowercase().ends_with(".md")
}

fn sanitize(repo: &str, path: &str) -> ViewerStateFile {
    let repo = repo.trim().to_string();
    let path = path.trim();
    ViewerStateFile {
        version: 1,
        path: if repo.is_empty() || !is_md_path(path) {
            String::new()
        } else {
            path.to_string()
        },
        repo,
    }
}

fn save_unlocked(data: &ViewerStateFile) -> Result<(), String> {
    let path = file_path()?;
    let value = json!({
        "version": data.version,
        "repo": data.repo,
        "path": data.path,
    });
    atomic_json::write_json(&path, &value)
}

fn load_unlocked() -> ViewerStateFile {
    let Ok(path) = file_path() else {
        return ViewerStateFile::default();
    };
    if !path.is_file() {
        return ViewerStateFile::default();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return ViewerStateFile::default();
    };
    match serde_json::from_str::<ViewerStateFile>(&text) {
        Ok(data) => sanitize(&data.repo, &data.path),
        Err(_) => ViewerStateFile::default(),
    }
}

fn as_json(data: &ViewerStateFile) -> serde_json::Value {
    json!({ "repo": data.repo, "path": data.path })
}

pub fn get_json() -> Result<serde_json::Value, String> {
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    Ok(as_json(&load_unlocked()))
}

pub fn set_json(repo: &str, path: &str) -> Result<serde_json::Value, String> {
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let data = sanitize(repo, path);
    save_unlocked(&data)?;
    Ok(as_json(&data))
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/viewer_state.rs"]
mod tests;
