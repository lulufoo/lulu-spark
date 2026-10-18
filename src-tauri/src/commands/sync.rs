use serde_json::Value;
use tauri::AppHandle;

use crate::services::{
    draft, docs,
};

#[tauri::command]
pub async fn save_comment_draft(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || draft::save_comment_draft(&payload))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_doc_draft(
    _app: AppHandle,
    temp_id: String,
    content: String,
) -> Result<Value, String> {
    Ok(draft::save_doc_draft_json(&temp_id, &content))
}

#[tauri::command]
pub fn clear_doc_draft(_app: AppHandle, temp_id: String) -> Result<Value, String> {
    Ok(draft::clear_doc_draft_json(&temp_id))
}

/// The payload with `workspace_id` filled in from the document id when the caller left it out.
fn with_workspace(mut payload: Value) -> Value {
    if payload.get("workspace_id").is_some() {
        return payload;
    }
    let id = payload.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    let found = crate::services::spark_data::open().and_then(|conn| crate::services::spark_data::find_doc(&conn, &id));
    if let Ok(Some(found)) = found {
        payload["workspace_id"] = Value::String(found.workspace.id);
    }
    payload
}

#[tauri::command]
pub async fn delete_entry(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || docs::delete_doc(&with_workspace(payload)))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn move_entry_project(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut payload = with_workspace(payload);
        if let Some(project) = payload.get("new_project").cloned() {
            payload["project_id"] = project;
        }
        docs::move_doc(&payload)
    })
        .await
        .map_err(|e| e.to_string())
}


