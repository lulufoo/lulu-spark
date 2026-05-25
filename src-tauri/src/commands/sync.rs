use serde_json::Value;
use tauri::AppHandle;

use crate::services::{
    corpus_git, draft, entry_admin, github_move, kb_git, kb_iterm, settle,
};

#[tauri::command]
pub async fn save_comment_draft(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || draft::save_comment_draft(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn corpus_git_commit(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || corpus_git::corpus_git_commit(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn corpus_git_pull(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || corpus_git::corpus_git_pull(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn corpus_git_revert(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || corpus_git::corpus_git_revert(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_git_commit(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || kb_git::kb_git_commit(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn kb_git_revert(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || kb_git::kb_git_revert(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_entry(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || entry_admin::delete_entry(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_entry_project(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || entry_admin::move_entry_project(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn gh_move_assets(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || github_move::gh_move_assets(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn settle_entry(_app: AppHandle, payload: Value) -> Result<Value, String> {
    let repo_root = crate::config::paths::repo_root().map_err(|e| format!("{e:?}"))?;
    tauri::async_runtime::spawn_blocking(move || settle::settle_entry(&repo_root, &payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_kb_in_iterm(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || kb_iterm::open_kb_in_iterm(&payload))
        .await
        .map_err(|e| e.to_string())
}
