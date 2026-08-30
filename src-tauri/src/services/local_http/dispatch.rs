use std::path::PathBuf;

use serde_json::{json, Value};
use tiny_http::Method;

use crate::services::notes::{create_note_content, create_note};
use crate::services::todo_task;
use crate::services::workbench_read::{
    get_notes_asset, get_notes_catalog_latest_per_topic, get_notes_file, get_notes_index,
    infer_github_user_url,
};

use super::archive::{handle_archive_post, handle_notes_files_post};
use super::bind::handle_bind_complete;
use super::read_later::{
    handle_read_later_delete, handle_read_later_get, handle_read_later_patch, handle_read_later_post,
};
use super::respond::{
    map_value_to_response, parse_query, respond_from_value, respond_json, respond_raw,
    respond_read_later_from_value, respond_with_cors_empty, todo_task_get_response_body,
};
use super::todo::{
    handle_todo_task_add_attachment_payload, handle_todo_task_add_comment_payload,
    handle_todo_task_add_sub_payload, handle_todo_task_complete_payload, handle_todo_task_create,
    handle_todo_task_delete_comment_payload, handle_todo_task_delete_payload,
    handle_todo_task_delete_sub_payload, handle_todo_task_get_attachment_payload,
    handle_todo_task_link_archive_payload, handle_todo_task_list_attachments_payload,
    handle_todo_task_list_comments_payload, handle_todo_task_post,
    handle_todo_task_set_status_payload, handle_todo_task_update_attachment_payload,
    handle_todo_task_update_comment_payload, handle_todo_task_update_payload,
    handle_todo_task_update_sub_payload, handle_todo_tasks_get, todo_wire, todo_wire_read,
};

pub(super) fn is_read_later_path(path: &str) -> bool {
    path == "/api/read-later" || path.starts_with("/api/read-later/")
}

pub(super) fn is_todo_api_path(path: &str) -> bool {
    path == "/api/todo-tasks"
        || path == "/api/todo-task"
        || path.starts_with("/api/todo-task-")
}

pub(super) fn respond_todo_api_gated(request: tiny_http::Request, path: &str, err: Value) {
    // GET /api/todo-tasks uses the CORS response path (same as the happy-path list handler).
    if path == "/api/todo-tasks" {
        respond_read_later_from_value(request, err);
    } else {
        respond_from_value(request, err);
    }
}

pub(super) fn handle_request(repo_root: &PathBuf, port: u16, request: tiny_http::Request) {
    let url = request.url().to_string();
    let path = url.split('?').next().unwrap_or("").to_string();

    if request.method() == &Method::Options {
        if is_read_later_path(&path) {
            respond_with_cors_empty(request, 204);
            return;
        }
        respond_json(request, 405, json!({ "error": "Method not allowed" }));
        return;
    }

    // Durable migration gate: todo_* HTTP stays closed until todo_tasks/.migration_gate_passed exists.
    // Re-checked per request from disk — never a process-local script exit code; no auto-migrate here.
    if (request.method() == &Method::Get || request.method() == &Method::Post)
        && is_todo_api_path(&path)
    {
        if let Err(err) = todo_task::ensure_todo_api_ungated() {
            respond_todo_api_gated(request, &path, err.into_wire());
            return;
        }
    }

    if request.method() == &Method::Patch {
        if let Some(id) = path.strip_prefix("/api/read-later/") {
            if !id.is_empty() && !id.contains('/') {
                handle_read_later_patch(request, id);
                return;
            }
        }
        respond_json(request, 405, json!({ "error": "Method not allowed" }));
        return;
    }

    if request.method() == &Method::Delete {
        if let Some(id) = path.strip_prefix("/api/read-later/") {
            if !id.is_empty() && !id.contains('/') {
                handle_read_later_delete(request, id);
                return;
            }
        }
        respond_json(request, 405, json!({ "error": "Method not allowed" }));
        return;
    }

    if request.method() == &Method::Post {
        match path.as_str() {
            "/api/read-later" => {
                handle_read_later_post(request);
                return;
            }
            "/api/notes-files" => {
                handle_notes_files_post(repo_root, request);
                return;
            }
            "/api/create-note" => {
                handle_archive_post(repo_root, request, create_note);
                return;
            }
            "/api/create-note-content" => {
                handle_archive_post(repo_root, request, create_note_content);
                return;
            }
            "/api/todo-task-create" => {
                handle_todo_task_create(request);
                return;
            }
            "/api/todo-task-delete" => {
                handle_todo_task_post(request, handle_todo_task_delete_payload);
                return;
            }
            "/api/todo-task-add-sub" => {
                handle_todo_task_post(request, handle_todo_task_add_sub_payload);
                return;
            }
            "/api/todo-task-update-sub" => {
                handle_todo_task_post(request, handle_todo_task_update_sub_payload);
                return;
            }
            "/api/todo-task-delete-sub" => {
                handle_todo_task_post(request, handle_todo_task_delete_sub_payload);
                return;
            }
            "/api/todo-task-complete" => {
                handle_todo_task_post(request, handle_todo_task_complete_payload);
                return;
            }
            "/api/todo-task-set-status" => {
                handle_todo_task_post(request, handle_todo_task_set_status_payload);
                return;
            }
            "/api/todo-task-link-archive" => {
                handle_todo_task_post(request, handle_todo_task_link_archive_payload);
                return;
            }
            "/api/todo-task-add-attachment" => {
                handle_todo_task_post(request, handle_todo_task_add_attachment_payload);
                return;
            }
            "/api/todo-task-list-attachments" => {
                handle_todo_task_post(request, handle_todo_task_list_attachments_payload);
                return;
            }
            "/api/todo-task-get-attachment" => {
                handle_todo_task_post(request, handle_todo_task_get_attachment_payload);
                return;
            }
            "/api/todo-task-update-attachment" => {
                handle_todo_task_post(request, handle_todo_task_update_attachment_payload);
                return;
            }
            "/api/todo-task-list-comments" => {
                handle_todo_task_post(request, handle_todo_task_list_comments_payload);
                return;
            }
            "/api/todo-task-add-comment" => {
                handle_todo_task_post(request, handle_todo_task_add_comment_payload);
                return;
            }
            "/api/todo-task-update-comment" => {
                handle_todo_task_post(request, handle_todo_task_update_comment_payload);
                return;
            }
            "/api/todo-task-delete-comment" => {
                handle_todo_task_post(request, handle_todo_task_delete_comment_payload);
                return;
            }
            "/api/todo-task-update" => {
                handle_todo_task_post(request, handle_todo_task_update_payload);
                return;
            }
            "/api/bind-complete" => {
                handle_bind_complete(request);
                return;
            }
            _ => {}
        }
    }

    if request.method() == &Method::Get {
        match path.as_str() {
            "/api/read-later" => {
                handle_read_later_get(request);
                return;
            }
            "/api/todo-tasks" => {
                handle_todo_tasks_get(request, &url);
                return;
            }
            "/api/todo-task-list-categories" => {
                let value = todo_wire(todo_task::list_todo_categories(), 200);
                let (status, body) = map_value_to_response(value);
                respond_raw(request, status, body);
                return;
            }
            "/api/todo-task" => {
                let params = parse_query(&url);
                let id = params.get("id").map(String::as_str).unwrap_or("");
                let value = todo_wire_read(todo_task::get_by_id(id));
                let (status, body) = todo_task_get_response_body(&value);
                respond_raw(request, status, body);
                return;
            }
            "/api/notes-catalog" => {
                let params = parse_query(&url);
                let mode = params.get("mode").map(String::as_str).unwrap_or("");
                if mode != "latest_per_topic" {
                    respond_json(
                        request,
                        400,
                        json!({ "error": "Unsupported mode; use mode=latest_per_topic" }),
                    );
                    return;
                }
                let value = get_notes_catalog_latest_per_topic(repo_root);
                respond_from_value(request, value);
                return;
            }
            "/api/notes-index" => {
                let value = get_notes_index(repo_root);
                respond_from_value(request, value);
                return;
            }
            "/api/notes-file" => {
                let params = parse_query(&url);
                let layer = params.get("layer").map(String::as_str).unwrap_or("");
                let common_path = params.get("path").map(String::as_str).unwrap_or("");
                if layer != "digest" {
                    respond_json(
                        request,
                        400,
                        json!({ "error": format!("Invalid layer: {layer}") }),
                    );
                    return;
                }
                let value = get_notes_file(repo_root, layer, common_path);
                respond_from_value(request, value);
                return;
            }
            "/api/notes-asset" => {
                let params = parse_query(&url);
                let layer = params.get("layer").map(String::as_str).unwrap_or("");
                let base = params.get("base").map(String::as_str).unwrap_or("");
                let href = params.get("href").map(String::as_str).unwrap_or("");
                let value = get_notes_asset(repo_root, layer, base, href);
                respond_from_value(request, value);
                return;
            }
            "/api/status" => {
                respond_json(
                    request,
                    200,
                    json!({
                        "ok": true,
                        "http_port": port,
                    }),
                );
                return;
            }
            "/api/infer-github-user-url" => {
                let params = parse_query(&url);
                let path = params.get("path").map(String::as_str).unwrap_or("");
                respond_from_value(request, infer_github_user_url(path));
                return;
            }
            _ => {
                respond_json(request, 404, json!({ "error": "Not found" }));
                return;
            }
        }
    }

    respond_json(request, 405, json!({ "error": "Method not allowed" }));
}
