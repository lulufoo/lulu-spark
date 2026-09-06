use serde_json::Value;
use tauri::AppHandle;

use crate::services::knowledge;

#[tauri::command]
pub fn add_kb_hide_pattern(_app: AppHandle, payload: Value) -> Result<Value, String> {
    let pattern = payload.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    knowledge::add_kb_hide_pattern(pattern)
}

#[tauri::command]
pub fn update_kb_hide_pattern(_app: AppHandle, payload: Value) -> Result<Value, String> {
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let pattern = payload.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
    knowledge::update_kb_hide_pattern(id, pattern)
}

#[tauri::command]
pub fn remove_kb_hide_pattern(_app: AppHandle, payload: Value) -> Result<Value, String> {
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    knowledge::remove_kb_hide_pattern(id)
}
