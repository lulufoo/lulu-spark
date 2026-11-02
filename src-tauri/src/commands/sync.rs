use serde_json::Value;
use tauri::{AppHandle, State};

use crate::services::doc_sync::{self, SyncGate};
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
    tauri::async_runtime::spawn_blocking(move || docs::delete_doc(&payload))
        .await
        .map_err(|e| e.to_string())
}

/// Syncs this machine's documents with the signed-in user's cloud and answers with the counts
/// (see [`doc_sync::summary`]). No arguments: the session is read from the vault in Rust.
#[tauri::command]
pub async fn sync_documents(gate: State<'_, SyncGate>) -> Result<Value, String> {
    let gate = gate.inner().clone();
    tauri::async_runtime::spawn_blocking(move || doc_sync::sync_now(&gate))
        .await
        .map_err(|e| e.to_string())
}

/// Asks the sync that is going on to stop (see [`doc_sync::cancel_now`]). It answers at once; the
/// sync's own answer, the one of `sync_documents`, comes when the transfer in progress is done.
#[tauri::command]
pub fn cancel_sync_documents(gate: State<'_, SyncGate>) -> Result<Value, String> {
    Ok(doc_sync::cancel_now(&gate))
}

/// Counts what a sync would send and bring back, without doing it (see [`doc_sync::check_now`]).
/// No arguments, and no gate: it only reads, so it may be asked while a sync is going on.
#[tauri::command]
pub async fn check_sync_documents() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(doc_sync::check_now)
        .await
        .map_err(|e| e.to_string())
}

/// Forwards `project_id` and optional `target_workspace_id` to [`docs::move_doc`].
#[tauri::command]
pub async fn move_entry_project(_app: AppHandle, payload: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        docs::move_doc(&with_workspace(payload))
    })
        .await
        .map_err(|e| e.to_string())
}



#[cfg(test)]
#[path = "../unit-tests/commands/sync.rs"]
mod tests;
