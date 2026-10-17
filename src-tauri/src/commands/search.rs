use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};
use tauri::State;

use crate::config::paths;
use crate::services::reindex::{all_status_json, run_all_reindex_blocking, ReindexState};

fn repo_root() -> Result<PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

/// Header "Rebuild index": every workspace's documents, index only (no `git pull`).
#[tauri::command]
pub fn reindex_all(state: State<'_, ReindexState>) -> Result<Value, String> {
    let root = repo_root()?;
    {
        let job = state.spark_job.lock().map_err(|e| e.to_string())?;
        if job.status == "running" {
            return Err("already running".to_string());
        }
    }
    let slot = Arc::clone(&state.spark_job);
    tauri::async_runtime::spawn_blocking(move || {
        let _ = run_all_reindex_blocking(&root, &slot);
    });
    Ok(json!({ "status": "running" }))
}

#[tauri::command]
pub fn get_reindex_all_status(state: State<'_, ReindexState>) -> Result<Value, String> {
    all_status_json(&state.spark_job)
}
