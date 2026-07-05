use serde_json::{json, Value};
use tauri::AppHandle;

use crate::services::read_later;

fn map_invoke_value(mut value: Value) -> Result<Value, String> {
    let status = value
        .get("_status")
        .and_then(|v| v.as_u64())
        .unwrap_or(200) as u16;
    if let Some(obj) = value.as_object_mut() {
        obj.remove("_status");
    }
    if status >= 400 {
        if let Some(obj) = value.as_object_mut() {
            obj.insert("_status".to_string(), json!(status));
        }
    }
    Ok(value)
}

pub fn create_read_later_json(url: &str, title: Option<&str>) -> Result<Value, String> {
    map_invoke_value(read_later::create_entry(url, title))
}

pub fn get_read_later_json() -> Result<Value, String> {
    Ok(read_later::list_entries())
}

pub fn mark_read_later_json(id: &str, read: bool) -> Result<Value, String> {
    map_invoke_value(read_later::mark_read(id, read))
}

pub fn delete_read_later_json(id: &str) -> Result<Value, String> {
    map_invoke_value(read_later::delete_entry(id))
}

#[tauri::command]
pub async fn create_read_later(
    _app: AppHandle,
    url: String,
    title: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        create_read_later_json(&url, title.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_read_later(_app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(get_read_later_json)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn mark_read_later(_app: AppHandle, id: String, read: bool) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || mark_read_later_json(&id, read))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_read_later(_app: AppHandle, id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || delete_read_later_json(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../unit-tests/commands/read_later.rs"]
mod read_later_tests;
