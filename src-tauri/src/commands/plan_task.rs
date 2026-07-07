use serde_json::Value;
use tauri::AppHandle;

use crate::services::plan_task;

pub fn get_plan_tasks_json() -> Result<Value, String> {
    match plan_task::list_all() {
        Value::Array(items) => Ok(Value::Array(items)),
        other => {
            let message = other
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to load plan tasks");
            Err(message.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_plan_tasks(_app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(get_plan_tasks_json)
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../unit-tests/commands/plan_task.rs"]
mod tests;
