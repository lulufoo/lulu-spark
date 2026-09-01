use serde_json::Value;
use tauri::AppHandle;

use crate::services::notes;

fn strip_ok_status(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        let status = obj.get("_status").and_then(|v| v.as_u64()).unwrap_or(200);
        if status < 400 {
            obj.remove("_status");
        }
    }
    value
}

#[tauri::command]
pub fn list_notes_categories(_app: AppHandle) -> Result<Value, String> {
    Ok(strip_ok_status(notes::list_notes_categories_value()))
}

#[tauri::command]
pub fn create_notes_category(_app: AppHandle, payload: Value) -> Result<Value, String> {
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(strip_ok_status(notes::create_notes_category_value(
        id,
        title,
        description,
    )))
}

#[tauri::command]
pub fn update_notes_category(_app: AppHandle, payload: Value) -> Result<Value, String> {
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let title = payload.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let description = payload
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    Ok(strip_ok_status(notes::update_notes_category_value(
        id,
        title,
        description,
    )))
}

#[tauri::command]
pub fn delete_notes_category(_app: AppHandle, payload: Value) -> Result<Value, String> {
    let id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("");
    Ok(strip_ok_status(notes::delete_notes_category_value(id)))
}
