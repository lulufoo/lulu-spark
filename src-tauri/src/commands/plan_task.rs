use serde_json::{json, Value};
use tauri::AppHandle;

use crate::services::plan_task;

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

pub fn create_plan_task_json(title: &str, sub_titles: Option<&[&str]>) -> Result<Value, String> {
    map_invoke_value(plan_task::create_master_with_subs(title, sub_titles))
}

pub fn delete_plan_task_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(plan_task::delete_master(master_task_id))
}

pub fn add_plan_sub_json(master_task_id: &str, title: &str) -> Result<Value, String> {
    map_invoke_value(plan_task::add_sub(master_task_id, title))
}

pub fn delete_plan_sub_json(master_task_id: &str, sub_task_id: &str) -> Result<Value, String> {
    map_invoke_value(plan_task::delete_sub(master_task_id, sub_task_id))
}

#[tauri::command]
pub async fn get_plan_tasks(_app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(get_plan_tasks_json)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_plan_task(
    _app: AppHandle,
    title: String,
    sub_titles: Option<Vec<String>>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let refs: Option<Vec<&str>> = sub_titles
            .as_ref()
            .map(|v| v.iter().map(String::as_str).collect());
        create_plan_task_json(&title, refs.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_plan_task(_app: AppHandle, master_task_id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || delete_plan_task_json(&master_task_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_plan_sub(
    _app: AppHandle,
    master_task_id: String,
    title: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || add_plan_sub_json(&master_task_id, &title))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_plan_sub(
    _app: AppHandle,
    master_task_id: String,
    sub_task_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        delete_plan_sub_json(&master_task_id, &sub_task_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../unit-tests/commands/plan_task.rs"]
mod tests;
