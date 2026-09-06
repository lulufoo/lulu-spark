use serde_json::Value;
use tauri::AppHandle;

use crate::services::knowledge;

#[tauri::command]
pub fn get_kb_viewer_state(_app: AppHandle) -> Result<Value, String> {
    knowledge::kb_viewer_state_json()
}

#[tauri::command]
pub fn set_kb_viewer_state(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    knowledge::set_kb_viewer_state(&repo, &path)
}
