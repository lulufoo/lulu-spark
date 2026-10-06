use serde_json::Value;
use tauri::AppHandle;

use crate::services::{
    draft, entry_admin,
};

#[tauri::command]
pub async fn save_comment_draft(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || draft::save_comment_draft(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_note_draft(
    _app: AppHandle,
    temp_id: String,
    content: String,
) -> Result<Value, String> {
    Ok(draft::save_note_draft_json(&temp_id, &content))
}

#[tauri::command]
pub fn clear_note_draft(_app: AppHandle, temp_id: String) -> Result<Value, String> {
    Ok(draft::clear_note_draft_json(&temp_id))
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


