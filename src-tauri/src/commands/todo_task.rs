use serde_json::{json, Value};
use tauri::AppHandle;

use crate::services::todo_task;

/// Locked master-task `status` wire values for list/get/create command exits.
pub(crate) const TODO_TASK_MASTER_STATUS_WIRE: &[&str] = &["incomplete", "complete", "abandoned"];

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

pub fn get_todo_tasks_json() -> Result<Value, String> {
    match todo_task::list_all() {
        Value::Array(items) => Ok(Value::Array(items)),
        other => {
            let message = other
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Failed to load todo tasks");
            Err(message.to_string())
        }
    }
}

pub fn create_todo_task_json(
    title: &str,
    sub_titles: Option<&[&str]>,
    plan_md: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::create_master_with_subs_and_todo(
        title, sub_titles, plan_md,
    ))
}

pub fn delete_todo_task_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_master(master_task_id))
}

pub fn add_todo_sub_json(master_task_id: &str, title: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::add_sub(master_task_id, title))
}

pub fn delete_todo_sub_json(master_task_id: &str, sub_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_sub(master_task_id, sub_task_id))
}

pub fn read_todo_md_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::read_todo_md(master_task_id))
}

pub fn update_todo_md_json(master_task_id: &str, plan_md: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::update_todo_md(master_task_id, plan_md))
}

pub fn complete_todo_json(
    master_task_id: &str,
    sub_task_id: Option<&str>,
) -> Result<Value, String> {
    map_invoke_value(todo_task::complete_todo(master_task_id, sub_task_id))
}

pub fn abandon_todo_sub_json(master_task_id: &str, sub_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::abandon_sub(master_task_id, sub_task_id))
}

pub fn update_todo_sub_json(
    master_task_id: &str,
    sub_task_id: &str,
    title: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::update_sub_title(master_task_id, sub_task_id, title))
}

pub fn update_todo_master_title_json(
    master_task_id: &str,
    title: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::update_master_title(master_task_id, title))
}

pub fn set_todo_master_status_json(
    master_task_id: &str,
    status: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::set_master_status(master_task_id, status))
}

pub fn add_todo_attachment_json(
    master_task_id: &str,
    file_name: &str,
    content: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::add_attachment(master_task_id, file_name, content))
}

pub fn list_todo_attachments_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::list_attachments(master_task_id))
}

pub fn read_todo_attachment_json(
    master_task_id: &str,
    file_name: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::read_attachment(master_task_id, file_name))
}

pub fn save_todo_attachment_json(
    master_task_id: &str,
    file_name: &str,
    content: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::save_attachment(master_task_id, file_name, content))
}

pub fn delete_todo_attachment_json(
    master_task_id: &str,
    file_name: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_attachment(master_task_id, file_name))
}

pub fn list_todo_comments_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::list_comments(master_task_id))
}

pub fn add_todo_comment_json(master_task_id: &str, body: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::add_comment(master_task_id, body))
}

pub fn update_todo_comment_json(
    master_task_id: &str,
    comment_id: &str,
    body: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::update_comment(master_task_id, comment_id, body))
}

pub fn delete_todo_comment_json(
    master_task_id: &str,
    comment_id: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_comment(master_task_id, comment_id))
}

#[tauri::command]
pub async fn get_todo_tasks(_app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(get_todo_tasks_json)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_todo_task(
    _app: AppHandle,
    title: String,
    sub_titles: Option<Vec<String>>,
    plan_md: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let refs: Option<Vec<&str>> = sub_titles
            .as_ref()
            .map(|v| v.iter().map(String::as_str).collect());
        let md = plan_md.as_deref().unwrap_or("");
        create_todo_task_json(&title, refs.as_deref(), md)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_todo_task(_app: AppHandle, master_task_id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || delete_todo_task_json(&master_task_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_todo_sub(
    _app: AppHandle,
    master_task_id: String,
    title: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || add_todo_sub_json(&master_task_id, &title))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_todo_sub(
    _app: AppHandle,
    master_task_id: String,
    sub_task_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        delete_todo_sub_json(&master_task_id, &sub_task_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn read_todo_md(_app: AppHandle, master_task_id: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let v = read_todo_md_json(&master_task_id)?;
        if let Some(status) = v.get("_status").and_then(|s| s.as_u64()) {
            if status >= 400 {
                let msg = v
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("read_todo_md failed");
                return Err(msg.to_string());
            }
        }
        Ok(v.get("todo_md")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_todo_md(
    _app: AppHandle,
    master_task_id: String,
    plan_md: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let v = update_todo_md_json(&master_task_id, &plan_md)?;
        if let Some(status) = v.get("_status").and_then(|s| s.as_u64()) {
            if status >= 400 {
                let msg = v
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("update_todo_md failed");
                return Err(msg.to_string());
            }
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn complete_todo(
    _app: AppHandle,
    master_task_id: String,
    sub_task_id: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        complete_todo_json(&master_task_id, sub_task_id.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn abandon_todo_sub(
    _app: AppHandle,
    master_task_id: String,
    sub_task_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        abandon_todo_sub_json(&master_task_id, &sub_task_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_todo_sub(
    _app: AppHandle,
    master_task_id: String,
    sub_task_id: String,
    title: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        update_todo_sub_json(&master_task_id, &sub_task_id, &title)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_todo_master_title(
    _app: AppHandle,
    master_task_id: String,
    title: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        update_todo_master_title_json(&master_task_id, &title)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_todo_master_status(
    _app: AppHandle,
    master_task_id: String,
    status: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        set_todo_master_status_json(&master_task_id, &status)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_todo_attachment(
    _app: AppHandle,
    master_task_id: String,
    file_name: String,
    content: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        add_todo_attachment_json(&master_task_id, &file_name, &content)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_todo_attachments(
    _app: AppHandle,
    master_task_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || list_todo_attachments_json(&master_task_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn read_todo_attachment(
    _app: AppHandle,
    master_task_id: String,
    file_name: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        read_todo_attachment_json(&master_task_id, &file_name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn save_todo_attachment(
    _app: AppHandle,
    master_task_id: String,
    file_name: String,
    content: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        save_todo_attachment_json(&master_task_id, &file_name, &content)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_todo_attachment(
    _app: AppHandle,
    master_task_id: String,
    file_name: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        delete_todo_attachment_json(&master_task_id, &file_name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn list_todo_comments(
    _app: AppHandle,
    master_task_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || list_todo_comments_json(&master_task_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_todo_comment(
    _app: AppHandle,
    master_task_id: String,
    body: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || add_todo_comment_json(&master_task_id, &body))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn update_todo_comment(
    _app: AppHandle,
    master_task_id: String,
    comment_id: String,
    body: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        update_todo_comment_json(&master_task_id, &comment_id, &body)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_todo_comment(
    _app: AppHandle,
    master_task_id: String,
    comment_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        delete_todo_comment_json(&master_task_id, &comment_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
#[path = "../unit-tests/commands/todo_task.rs"]
mod tests;
