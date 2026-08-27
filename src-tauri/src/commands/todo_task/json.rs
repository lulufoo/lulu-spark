use serde_json::{json, Value};

use crate::services::todo_task;

/// Locked master-task `status` wire values for list/get/create command exits.
pub(crate) const TODO_TASK_MASTER_STATUS_WIRE: &[&str] = &["incomplete", "complete", "abandoned"];

fn map_invoke_value(
    result: Result<Value, todo_task::TodoError>,
    ok_status: u16,
) -> Result<Value, String> {
    let mut value = todo_task::into_wire(result, ok_status);
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
        Ok(Value::Array(items)) => Ok(Value::Array(items)),
        Ok(_) => Err("Failed to load todo tasks".to_string()),
        Err(err) => Err(err.message),
    }
}

pub fn create_todo_task_json(
    title: &str,
    sub_titles: Option<&[&str]>,
    plan_md: &str,
) -> Result<Value, String> {
    map_invoke_value(
        todo_task::create_master_with_subs_and_todo(title, sub_titles, plan_md),
        201,
    )
}

pub fn delete_todo_task_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_master(master_task_id), 200)
}

pub fn add_todo_sub_json(
    master_task_id: &str,
    title: &str,
    content: Option<&str>,
) -> Result<Value, String> {
    map_invoke_value(todo_task::add_sub(master_task_id, title, content), 201)
}

pub fn delete_todo_sub_json(master_task_id: &str, sub_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_sub(master_task_id, sub_task_id), 200)
}

pub fn read_todo_md_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::read_todo_md(master_task_id), 200)
}

pub fn update_todo_md_json(master_task_id: &str, plan_md: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::update_todo_md(master_task_id, plan_md), 200)
}

pub fn complete_todo_json(
    master_task_id: &str,
    sub_task_id: Option<&str>,
) -> Result<Value, String> {
    map_invoke_value(todo_task::complete_todo(master_task_id, sub_task_id), 200)
}

pub fn abandon_todo_sub_json(master_task_id: &str, sub_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::abandon_sub(master_task_id, sub_task_id), 200)
}

pub fn update_todo_sub_json(
    master_task_id: &str,
    sub_task_id: &str,
    title: &str,
    content: Option<&str>,
) -> Result<Value, String> {
    map_invoke_value(
        todo_task::update_sub_title(master_task_id, sub_task_id, title, content),
        200,
    )
}

pub fn update_todo_master_title_json(
    master_task_id: &str,
    title: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::update_master_title(master_task_id, title), 200)
}

pub fn set_todo_master_status_json(
    master_task_id: &str,
    status: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::set_master_status(master_task_id, status), 200)
}

pub fn add_todo_attachment_json(
    master_task_id: &str,
    source_path: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::add_attachment(master_task_id, source_path), 201)
}

pub fn stage_todo_attachment_source_json(
    preferred_name: &str,
    content: &str,
) -> Result<Value, String> {
    map_invoke_value(
        todo_task::stage_attachment_source(preferred_name, content),
        201,
    )
}

pub fn list_todo_attachments_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::list_attachments(master_task_id), 200)
}

pub fn read_todo_attachment_json(
    master_task_id: &str,
    file_name: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::read_attachment(master_task_id, file_name), 200)
}

pub fn save_todo_attachment_json(
    master_task_id: &str,
    file_name: &str,
    source_path: &str,
) -> Result<Value, String> {
    map_invoke_value(
        todo_task::save_attachment(master_task_id, file_name, source_path),
        200,
    )
}

pub fn delete_todo_attachment_json(
    master_task_id: &str,
    file_name: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_attachment(master_task_id, file_name), 200)
}

pub fn list_todo_comments_json(master_task_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::list_comments(master_task_id), 200)
}

pub fn add_todo_comment_json(master_task_id: &str, body: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::add_comment(master_task_id, body), 201)
}

pub fn update_todo_comment_json(
    master_task_id: &str,
    comment_id: &str,
    body: &str,
) -> Result<Value, String> {
    map_invoke_value(
        todo_task::update_comment(master_task_id, comment_id, body),
        200,
    )
}

pub fn delete_todo_comment_json(
    master_task_id: &str,
    comment_id: &str,
) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_comment(master_task_id, comment_id), 200)
}

pub fn list_todo_categories_json() -> Result<Value, String> {
    map_invoke_value(todo_task::list_todo_categories(), 200)
}

pub fn create_todo_category_json(name: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::create_todo_category(name), 201)
}

pub fn delete_todo_category_json(category_id: &str) -> Result<Value, String> {
    map_invoke_value(todo_task::delete_todo_category(category_id), 200)
}

pub fn set_todo_category_json(
    master_task_id: &str,
    category_id: &str,
) -> Result<Value, String> {
    map_invoke_value(
        todo_task::set_master_category(master_task_id, category_id),
        200,
    )
}
