//! Workspace and project management for the app: thin wrappers over the docs service.
//!
//! Business errors stay in the returned Value (`error` + `_status`); a success drops the
//! `_status` marker so the frontend sees the plain payload.

use serde_json::Value;
use tauri::AppHandle;

use crate::services::docs;

fn strip_ok_status(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        let status = obj.get("_status").and_then(|v| v.as_u64()).unwrap_or(200);
        if status < 400 {
            obj.remove("_status");
        }
    }
    value
}

async fn run(call: impl FnOnce() -> Value + Send + 'static) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || strip_ok_status(call()))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_workspaces(_app: AppHandle) -> Result<Value, String> {
    run(|| docs::list_workspaces()).await
}

#[tauri::command]
pub async fn create_workspace(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::create_workspace(&payload)).await
}

#[tauri::command]
pub async fn rename_workspace(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::rename_workspace(&payload)).await
}

#[tauri::command]
pub async fn set_workspace_view_mode(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::set_view_mode(&payload)).await
}

#[tauri::command]
pub async fn delete_workspace(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::delete_workspace(&payload)).await
}

#[tauri::command]
pub async fn list_projects(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::list_projects(&payload)).await
}

#[tauri::command]
pub async fn create_project(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::create_project(&payload)).await
}

#[tauri::command]
pub async fn update_project(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::update_project(&payload)).await
}

#[tauri::command]
pub async fn delete_project(_app: AppHandle, payload: Value) -> Result<Value, String> {
    run(move || docs::delete_project(&payload)).await
}
