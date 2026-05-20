use serde_json::Value;
use tauri::AppHandle;

use crate::config::paths;
use crate::services::{annotation, entry_write, kb_write};

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn set_done(_app: AppHandle, common_path: String, done: bool) -> Result<Value, String> {
    Ok(annotation::set_done(&repo_root()?, &common_path, done))
}

#[tauri::command]
pub fn set_importance(
    _app: AppHandle,
    common_path: String,
    importance: Option<String>,
) -> Result<Value, String> {
    Ok(annotation::set_importance(
        &repo_root()?,
        &common_path,
        importance,
    ))
}

#[tauri::command]
pub fn update_links(
    _app: AppHandle,
    common_path: String,
    links: Value,
) -> Result<Value, String> {
    Ok(annotation::update_links(&repo_root()?, &common_path, links))
}

#[tauri::command]
pub fn update_comments(
    _app: AppHandle,
    common_path: String,
    layer: String,
    comment: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(annotation::update_comments(
        &repo_root()?,
        &common_path,
        &layer,
        comment,
        ts,
    ))
}

#[tauri::command]
pub fn reorder_comments(
    _app: AppHandle,
    common_path: String,
    layer: String,
    ids: Vec<String>,
) -> Result<Value, String> {
    Ok(annotation::reorder_comments(
        &repo_root()?,
        &common_path,
        &layer,
        ids,
    ))
}

#[tauri::command]
pub fn update_highlights(
    _app: AppHandle,
    common_path: String,
    layer: String,
    highlight: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(annotation::update_highlights(
        &repo_root()?,
        &common_path,
        &layer,
        highlight,
        ts,
    ))
}

#[tauri::command]
pub fn save_entry(
    _app: AppHandle,
    layer: String,
    common_path: String,
    content: String,
) -> Result<Value, String> {
    Ok(entry_write::save_entry(
        &repo_root()?,
        layer,
        common_path,
        content,
    ))
}

#[tauri::command]
pub fn kb_save(
    _app: AppHandle,
    repo: String,
    path: String,
    content: String,
) -> Result<Value, String> {
    Ok(kb_write::kb_save(&repo_root()?, repo, path, content))
}

#[tauri::command]
pub fn kb_update_comments(
    _app: AppHandle,
    repo: String,
    path: String,
    comment: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(kb_write::kb_update_comments(
        &repo_root()?,
        repo,
        path,
        comment,
        ts,
    ))
}

#[tauri::command]
pub fn kb_reorder_comments(
    _app: AppHandle,
    repo: String,
    path: String,
    ids: Vec<String>,
) -> Result<Value, String> {
    Ok(kb_write::kb_reorder_comments(&repo_root()?, repo, path, ids))
}

#[tauri::command]
pub fn kb_update_highlights(
    _app: AppHandle,
    repo: String,
    path: String,
    highlight: Value,
    ts: String,
) -> Result<Value, String> {
    Ok(kb_write::kb_update_highlights(
        &repo_root()?,
        repo,
        path,
        highlight,
        ts,
    ))
}

#[tauri::command]
pub fn kb_update_links(
    _app: AppHandle,
    repo: String,
    path: String,
    links: Value,
) -> Result<Value, String> {
    Ok(kb_write::kb_update_links(&repo_root()?, repo, path, links))
}
