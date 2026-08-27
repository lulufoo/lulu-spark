use serde_json::Value;
use tauri::AppHandle;

use super::json::*;

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
    content: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        add_todo_sub_json(&master_task_id, &title, content.as_deref())
    })
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
    content: Option<String>,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        update_todo_sub_json(&master_task_id, &sub_task_id, &title, content.as_deref())
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
pub async fn stage_todo_attachment_source(
    _app: AppHandle,
    preferred_name: String,
    content: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        stage_todo_attachment_source_json(&preferred_name, &content)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn add_todo_attachment(
    _app: AppHandle,
    master_task_id: String,
    source_path: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        add_todo_attachment_json(&master_task_id, &source_path)
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
    source_path: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        save_todo_attachment_json(&master_task_id, &file_name, &source_path)
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

#[tauri::command]
pub async fn list_todo_categories(_app: AppHandle) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(list_todo_categories_json)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_todo_category(_app: AppHandle, name: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || create_todo_category_json(&name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_todo_category(
    _app: AppHandle,
    category_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || delete_todo_category_json(&category_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn set_todo_category(
    _app: AppHandle,
    master_task_id: String,
    category_id: String,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        set_todo_category_json(&master_task_id, &category_id)
    })
    .await
    .map_err(|e| e.to_string())?
}
