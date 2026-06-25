use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{json, Value};
use tauri::{AppHandle, State};

use crate::config::paths;
use crate::services::reindex::{
    finish_job_error, finish_job_success, run_kb_sync_and_index_blocking,
    run_knowledge_pull_and_reindex, run_knowledge_reindex_blocking,
    run_workbench_reindex_blocking, set_job_log, start_job, ReindexState,
};

fn repo_root() -> Result<PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn reindex_workbench(
    app: AppHandle,
    state: State<'_, ReindexState>,
) -> Result<Value, String> {
    let root = repo_root()?;
    start_job(&state.workbench_job, "启动中…")?;
    let slot = state.workbench_job.clone();
    let repo_root = root.clone();
    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            run_workbench_reindex_blocking(&repo_root)
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

#[tauri::command]
pub fn get_reindex_workbench_status(state: State<'_, ReindexState>) -> Result<Value, String> {
    let job = state
        .workbench_job
        .lock()
        .map_err(|e| e.to_string())?;
    Ok(job.to_json())
}

#[tauri::command]
pub fn reindex_knowledge(
    app: AppHandle,
    state: State<'_, ReindexState>,
) -> Result<Value, String> {
    let root = repo_root()?;
    start_job(&state.knowledge_job, "启动中…")?;
    let slot = state.knowledge_job.clone();
    let slot_blocking = Arc::clone(&slot);
    let repo_root = root.clone();
    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            run_knowledge_reindex_blocking(&repo_root, Some(&slot_blocking))
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
    if repo_owned.is_empty() || !repo_owned.contains('/') {
        return Ok(json!({ "error": "invalid repo format" }));
    }
    let repo_name = repo_owned
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();

    start_job(
        &state.knowledge_job,
        &format!("准备同步 {repo_name}…"),
    )?;
    let slot = state.knowledge_job.clone();
    let repo_root = root.clone();
    let repo_for_task = repo_owned.clone();
    let repo_name_log = repo_name.clone();
    tauri::async_runtime::spawn(async move {
        set_job_log(&slot, format!("同步 {repo_name_log}…"));
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

#[tauri::command]
pub fn sync_knowledge_corpus(
    app: AppHandle,
    state: State<'_, ReindexState>,
) -> Result<Value, String> {
    let root = repo_root()?;
    start_job(&state.knowledge_job, "启动中…")?;
    let slot = state.knowledge_job.clone();
    let slot_blocking = Arc::clone(&slot);
    let repo_root = root.clone();
    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            run_knowledge_pull_and_reindex(&repo_root, Some(&slot_blocking))
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
