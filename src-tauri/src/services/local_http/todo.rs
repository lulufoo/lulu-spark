use serde_json::{json, Value};

use crate::services::todo_task;

use super::respond::{
    map_value_to_response, parse_query, read_json_body, respond_from_value, respond_json,
    respond_read_later_from_value, respond_with_cors, todo_task_get_response_body,
    todo_tasks_list_response_body,
};

pub(super) fn todo_wire(result: Result<Value, todo_task::TodoError>, ok_status: u16) -> Value {
    todo_task::into_wire(result, ok_status)
}

pub(super) fn todo_wire_read(result: Result<Value, todo_task::TodoError>) -> Value {
    todo_task::into_wire_read(result)
}
pub(super) fn handle_todo_tasks_get(request: tiny_http::Request, url: &str) {
    let value = todo_wire_read(todo_task::list_all());
    if !value.is_array() {
        respond_read_later_from_value(request, value);
        return;
    }
    let params = parse_query(url);
    let filtered = match params
        .get("category_id")
        .map(String::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => value,
        Some(category_id) => Value::Array(
            value
                .as_array()
                .into_iter()
                .flatten()
                .filter(|t| t.get("category_id").and_then(|v| v.as_str()) == Some(category_id))
                .cloned()
                .collect(),
        ),
    };
    let body = todo_tasks_list_response_body(&filtered);
    respond_with_cors(request, 200, body);
}
pub(super) fn handle_todo_task_post(
    mut request: tiny_http::Request,
    handler: fn(&Value) -> Value,
) {
    let payload = match read_json_body(&mut request) {
        Ok(v) => v,
        Err(err) => {
            respond_json(request, 400, err);
            return;
        }
    };
    respond_from_value(request, handler(&payload));
}

pub(super) fn handle_todo_task_delete_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    todo_wire(todo_task::delete_master(master_task_id), 200)
}

pub(super) fn handle_todo_task_add_sub_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(title) = payload.get("title").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing title", "_status": 400 });
    };
    let content = payload.get("content").and_then(|v| v.as_str());
    todo_wire(todo_task::add_sub(master_task_id, title, content), 201)
}

pub(super) fn handle_todo_task_update_sub_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(sub_task_id) = payload.get("sub_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing sub_task_id", "_status": 400 });
    };
    let Some(title) = payload.get("title").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing title", "_status": 400 });
    };
    let content = payload.get("content").map(|v| v.as_str().unwrap_or(""));
    todo_wire(
        todo_task::update_sub_title(master_task_id, sub_task_id, title, content),
        200,
    )
}

pub(super) fn handle_todo_task_delete_sub_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(sub_task_id) = payload.get("sub_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing sub_task_id", "_status": 400 });
    };
    todo_wire(todo_task::delete_sub(master_task_id, sub_task_id), 200)
}

pub(super) fn handle_todo_task_complete_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let sub_task_id = payload.get("sub_task_id").and_then(|v| v.as_str());
    todo_wire(todo_task::complete_todo(master_task_id, sub_task_id), 200)
}

pub(super) fn handle_todo_task_set_status_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(status) = payload.get("status").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing status", "_status": 400 });
    };
    todo_wire(todo_task::set_master_status(master_task_id, status), 200)
}

pub(super) fn handle_todo_task_link_archive_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(sub_task_id) = payload.get("sub_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing sub_task_id", "_status": 400 });
    };
    let Some(archive_id) = payload.get("archive_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing archive_id", "_status": 400 });
    };
    todo_wire(
        todo_task::link_archive(master_task_id, sub_task_id, archive_id),
        200,
    )
}

pub(super) fn handle_todo_task_add_attachment_payload(payload: &Value) -> Value {
    if payload.get("content").is_some() {
        return json!({
            "error": "content is not supported; use source_path",
            "_status": 400
        });
    }
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(source_path) = payload.get("source_path").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing source_path", "_status": 400 });
    };
    todo_wire(todo_task::add_attachment(master_task_id, source_path), 201)
}

pub(super) fn handle_todo_task_list_attachments_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    todo_wire(todo_task::list_attachments(master_task_id), 200)
}

pub(super) fn handle_todo_task_get_attachment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(file_name) = payload.get("file_name").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing file_name", "_status": 400 });
    };
    todo_wire(todo_task::read_attachment(master_task_id, file_name), 200)
}

pub(super) fn handle_todo_task_update_attachment_payload(payload: &Value) -> Value {
    if payload.get("content").is_some() {
        return json!({
            "error": "content is not supported; use source_path",
            "_status": 400
        });
    }
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(file_name) = payload.get("file_name").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing file_name", "_status": 400 });
    };
    let Some(source_path) = payload.get("source_path").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing source_path", "_status": 400 });
    };
    todo_wire(
        todo_task::save_attachment(master_task_id, file_name, source_path),
        200,
    )
}

pub(super) fn handle_todo_task_list_comments_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    todo_wire(todo_task::list_comments(master_task_id), 200)
}

pub(super) fn handle_todo_task_add_comment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(body) = payload.get("body").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing body", "_status": 400 });
    };
    todo_wire(todo_task::add_comment(master_task_id, body), 201)
}

pub(super) fn handle_todo_task_update_comment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(comment_id) = payload.get("comment_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing comment_id", "_status": 400 });
    };
    let Some(body) = payload.get("body").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing body", "_status": 400 });
    };
    todo_wire(
        todo_task::update_comment(master_task_id, comment_id, body),
        200,
    )
}

pub(super) fn handle_todo_task_delete_comment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(comment_id) = payload.get("comment_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing comment_id", "_status": 400 });
    };
    todo_wire(todo_task::delete_comment(master_task_id, comment_id), 200)
}

pub(super) fn handle_todo_task_update_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };

    let title = match payload.get("title") {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(_) => return json!({ "error": "Invalid title", "_status": 400 }),
    };
    let todo_md = match payload.get("todo_md") {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(_) => return json!({ "error": "Invalid todo_md", "_status": 400 }),
    };
    let category_id = match payload.get("category_id") {
        None => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(_) => return json!({ "error": "Invalid category_id", "_status": 400 }),
    };
    if title.is_none() && todo_md.is_none() && category_id.is_none() {
        // Keep legacy wording when no updatable field is present (pre-category clients).
        return json!({ "error": "Missing title or todo_md", "_status": 400 });
    }

    // category_id alone → set category; otherwise update title/body first, then optional category.
    if title.is_none() && todo_md.is_none() {
        return todo_wire(
            todo_task::set_master_category(master_task_id, category_id.unwrap_or("")),
            200,
        );
    }

    let updated = todo_wire(todo_task::update_master_fields(master_task_id, title, todo_md), 200);
    if updated.get("_status").and_then(|v| v.as_u64()).unwrap_or(200) >= 400 {
        return updated;
    }
    match category_id {
        Some(cid) => todo_wire(todo_task::set_master_category(master_task_id, cid), 200),
        None => updated,
    }
}

pub(super) fn handle_todo_task_create(mut request: tiny_http::Request) {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        respond_json(request, 400, json!({ "error": "Failed to read body" }));
        return;
    }
    let payload: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            respond_json(request, 400, json!({ "error": format!("Invalid JSON: {e}") }));
            return;
        }
    };
    let Some(title) = payload.get("title").and_then(|v| v.as_str()) else {
        respond_json(request, 400, json!({ "error": "Missing title" }));
        return;
    };

    let todo_md = match payload.get("todo_md") {
        None | Some(Value::Null) => {
            respond_json(request, 400, json!({ "error": "Missing todo_md" }));
            return;
        }
        Some(Value::String(s)) => {
            if s.trim().is_empty() {
                respond_json(request, 400, json!({ "error": "Missing todo_md" }));
                return;
            }
            s.as_str()
        }
        Some(_) => {
            respond_json(request, 400, json!({ "error": "Invalid todo_md" }));
            return;
        }
    };

    let category_id = match payload.get("category_id") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.as_str()),
        Some(_) => {
            respond_json(request, 400, json!({ "error": "Invalid category_id" }));
            return;
        }
    };

    let sub_titles: Option<Vec<String>> = match payload.get("sub_titles") {
        None | Some(Value::Null) => None,
        Some(Value::Array(arr)) if arr.is_empty() => None,
        Some(Value::Array(arr)) => {
            let mut subs = Vec::with_capacity(arr.len());
            for item in arr {
                let Some(raw) = item.as_str() else {
                    respond_json(request, 400, json!({ "error": "Invalid sub_titles element" }));
                    return;
                };
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    respond_json(request, 400, json!({ "error": "Invalid sub_titles element" }));
                    return;
                }
                subs.push(trimmed.to_string());
            }
            Some(subs)
        }
        Some(_) => {
            respond_json(request, 400, json!({ "error": "Invalid sub_titles" }));
            return;
        }
    };

    let value = match &sub_titles {
        Some(subs) => {
            let refs: Vec<&str> = subs.iter().map(String::as_str).collect();
            todo_wire(
                todo_task::create_master_with_category(title, Some(&refs), todo_md, category_id),
                201,
            )
        }
        None => todo_wire(
            todo_task::create_master_with_category(title, None, todo_md, category_id),
            201,
        ),
    };
    respond_from_value(request, value);
}
