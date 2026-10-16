use serde_json::Value;
use tauri::AppHandle;

use crate::config::paths;
use crate::services::search_index::{cache_dir_or_err, search_desktop_docs};
use crate::services::tags_registry;
use crate::services::spark_read;

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub async fn search_spark(
    _app: AppHandle,
    q: String,
    limit: Option<u32>,
    workspace: Option<String>,
) -> Result<Value, String> {
    let _ = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || {
        let cache = cache_dir_or_err()?;
        Ok(search_desktop_docs(&cache, &q, limit, workspace.as_deref()))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_annotations(_app: AppHandle) -> Result<Value, String> {
    Ok(spark_read::get_annotations_summary(&repo_root()?))
}

#[tauri::command]
pub fn get_tags_registry(_app: AppHandle) -> Result<Value, String> {
    Ok(tags_registry::read_registry())
}

#[tauri::command]
pub fn get_annotation(_app: AppHandle, id: String) -> Result<Value, String> {
    Ok(spark_read::get_annotation(&repo_root()?, &id))
}

#[tauri::command]
pub fn get_draft(_app: AppHandle, id: String) -> Result<Value, String> {
    Ok(spark_read::get_draft(&repo_root()?, &id))
}

#[tauri::command]
pub fn get_note_draft(_app: AppHandle, temp_id: String) -> Result<Value, String> {
    Ok(crate::services::draft::load_note_draft_json(&temp_id))
}

#[tauri::command]
pub fn get_doc_highlights(_app: AppHandle, key: String) -> Result<Value, String> {
    Ok(crate::services::doc_highlights::get_doc_highlights(&key))
}

#[tauri::command]
pub fn get_config(_app: AppHandle) -> Result<Value, String> {
    Ok(spark_read::get_config(&repo_root()?))
}

#[tauri::command]
pub async fn fetch_link_title(_app: AppHandle, url: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::link_title::fetch_link_title(&url)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_notes_index(_app: AppHandle, workspace: Option<String>) -> Result<Value, String> {
    Ok(spark_read::get_notes_index(workspace.as_deref()))
}

#[tauri::command]
pub fn get_notes_file(
    _app: AppHandle,
    layer: String,
    id: String,
    lang: Option<String>,
) -> Result<Value, String> {
    Ok(spark_read::get_notes_file(&layer, &id, lang.as_deref()))
}

/// Validate and resolve a `note:<id>` link target. Index entry only; no body, no absolute path.
#[tauri::command]
pub fn resolve_note_for_open(_app: AppHandle, id: String) -> Result<Value, String> {
    Ok(spark_read::resolve_note_for_open(&id))
}

#[tauri::command]
pub fn read_abs_file(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(crate::services::abs_file::read_abs_file(&path))
}

#[tauri::command]
pub fn get_notes_asset(
    _app: AppHandle,
    layer: String,
    id: String,
    href: String,
    lang: Option<String>,
) -> Result<Value, String> {
    Ok(spark_read::get_notes_asset(&layer, &id, &href, lang.as_deref()))
}

pub fn get_message_channel_unread_json(channel: &str) -> Result<bool, String> {
    Ok(crate::services::message_center::channel_unread(channel))
}

#[tauri::command]
pub fn get_message_channel_unread(_app: AppHandle, channel: String) -> Result<bool, String> {
    get_message_channel_unread_json(&channel)
}

#[cfg(test)]
#[path = "../unit-tests/commands/message_center.rs"]
mod message_center_tests;
