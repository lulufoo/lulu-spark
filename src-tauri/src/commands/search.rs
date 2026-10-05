use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};
use tauri::{AppHandle, State};

use crate::config::paths;
use crate::services::reindex::{
    all_status_json, finish_job_error, finish_job_success, run_all_reindex_blocking,
    run_kb_sync_and_index_blocking, set_job_log, start_job,
    ReindexState,
};

fn repo_root() -> Result<PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

/// Header "Rebuild index": notes then knowledge, index only (no `git pull`).
#[tauri::command]
pub fn reindex_all(state: State<'_, ReindexState>) -> Result<Value, String> {
    let root = repo_root()?;
    {
        let wb = state.spark_job.lock().map_err(|e| e.to_string())?;
        let kb = state.knowledge_job.lock().map_err(|e| e.to_string())?;
        if wb.status == "running" || kb.status == "running" {
            return Err("already running".to_string());
        }
    }
    let wb = Arc::clone(&state.spark_job);
    let kb = Arc::clone(&state.knowledge_job);
    tauri::async_runtime::spawn_blocking(move || {
        let _ = run_all_reindex_blocking(&root, &wb, &kb);
    });
    Ok(json!({ "status": "running" }))
}

#[tauri::command]
pub fn get_reindex_all_status(state: State<'_, ReindexState>) -> Result<Value, String> {
    all_status_json(&state.spark_job, &state.knowledge_job)
}

#[tauri::command]
pub fn get_reindex_status(state: State<'_, ReindexState>) -> Result<Value, String> {
    let job = state.knowledge_job.lock().map_err(|e| e.to_string())?;
    Ok(job.to_json())
}

#[tauri::command]
pub fn reindex_kb_repo(
    app: AppHandle,
    state: State<'_, ReindexState>,
    repo: String,
) -> Result<Value, String> {
    let root = repo_root()?;
    let repo_owned = repo.trim().to_string();
    if repo_owned.is_empty() {
        return Ok(json!({ "error": "invalid directory name" }));
    }
    let repo_name = repo_owned
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();

    start_job(
        &state.knowledge_job,
        &format!("准备重建索引 {repo_name}…"),
    )?;
    let slot = state.knowledge_job.clone();
    let repo_root = root.clone();
    let repo_for_task = repo_owned.clone();
    let repo_name_log = repo_name.clone();
    tauri::async_runtime::spawn(async move {
        set_job_log(&slot, format!("重建索引 {repo_name_log}…"));
        let result = tauri::async_runtime::spawn_blocking(move || {
            run_kb_sync_and_index_blocking(&repo_root, &repo_for_task)
        })
        .await;
        match result {
            Ok(Ok(log)) => finish_job_success(&slot, log),
            Ok(Err(e)) => finish_job_error(&slot, e),
            Err(e) => finish_job_error(&slot, e.to_string()),
        }
    });
    let _ = app;
    Ok(json!({ "status": "running" }))
}
