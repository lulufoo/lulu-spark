//! Localhost HTTP API for MCP sidecar proxy (`GET/POST /api/corpus-*`, `/api/archive-*`, `/api/read-later*`, `/api/todo-tasks`, `/api/status`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use serde_json::{json, Value};
use tiny_http::{Header, Method, Response, Server, StatusCode};

use crate::services::archive_write::{archive_digest, archive_document};
use crate::services::todo_task;
use crate::services::read_later;
use crate::services::workbench_read::{
    get_corpus_asset, get_corpus_catalog_latest_per_topic, get_corpus_file, get_corpus_files_by_ids,
    get_corpus_index,
};

pub const DEFAULT_HTTP_PORT: u16 = 8765;

/// Locked master-task `status` wire values for `/api/todo-tasks`, `/api/todo-task`, `/api/todo-task-create`.
pub(crate) const TODO_TASK_MASTER_STATUS_WIRE: &[&str] = &["incomplete", "complete", "abandoned"];

pub struct LocalHttpHandle {
    server: Arc<Server>,
    join: JoinHandle<()>,
}

#[derive(Debug)]
pub enum LocalHttpError {
    BindFailed(String),
}

pub struct LocalHttpState {
    handle: Mutex<Option<LocalHttpHandle>>,
    http_ready: Arc<AtomicBool>,
}

impl LocalHttpState {
    pub fn new() -> Self {
        Self {
            handle: Mutex::new(None),
            http_ready: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.http_ready.load(Ordering::SeqCst)
    }

    pub fn try_start(&self, repo_root: PathBuf, port: u16) {
        match start(repo_root, port) {
            Ok(handle) => {
                if let Ok(mut guard) = self.handle.lock() {
                    *guard = Some(handle);
                    self.http_ready.store(true, Ordering::SeqCst);
                }
            }
            Err(LocalHttpError::BindFailed(msg)) => {
                eprintln!("[local_http] bind failed on port {port}: {msg}");
                self.http_ready.store(false, Ordering::SeqCst);
            }
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                stop(handle);
            }
        }
        self.http_ready.store(false, Ordering::SeqCst);
    }
}

pub fn start(repo_root: PathBuf, port: u16) -> Result<LocalHttpHandle, LocalHttpError> {
    let addr = format!("127.0.0.1:{port}");
    let server = Server::http(&addr).map_err(|e| LocalHttpError::BindFailed(e.to_string()))?;
    let server = Arc::new(server);
    let server_for_thread = Arc::clone(&server);
    let join = thread::spawn(move || {
        for request in server_for_thread.incoming_requests() {
            handle_request(&repo_root, port, request);
        }
    });
    Ok(LocalHttpHandle { server, join })
}

pub fn stop(handle: LocalHttpHandle) {
    handle.server.unblock();
    let _ = handle.join.join();
}

pub(crate) fn map_value_to_response(value: Value) -> (u16, String) {
    let status = value
        .get("_status")
        .and_then(|v| v.as_u64())
        .unwrap_or(200) as u16;
    let mut body = value;
    if let Some(obj) = body.as_object_mut() {
        obj.remove("_status");
    }
    let json = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    (status, json)
}

/// Serialize `todo_task::list_all` for HTTP GET `/api/todo-tasks` (includes `todo_md`, `migration_error`).
pub(crate) fn todo_tasks_list_response_body(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "[]".to_string())
}

/// Serialize `todo_task::get_by_id` for HTTP GET `/api/todo-task` (includes `todo_md`, `migration_error`).
pub(crate) fn todo_task_get_response_body(value: &Value) -> (u16, String) {
    map_value_to_response(value.clone())
}

fn is_read_later_path(path: &str) -> bool {
    path == "/api/read-later" || path.starts_with("/api/read-later/")
}

fn is_todo_api_path(path: &str) -> bool {
    path == "/api/todo-tasks"
        || path == "/api/todo-task"
        || path.starts_with("/api/todo-task-")
}

fn respond_todo_api_gated(request: tiny_http::Request, path: &str, err: Value) {
    // GET /api/todo-tasks uses the CORS response path (same as the happy-path list handler).
    if path == "/api/todo-tasks" {
        respond_read_later_from_value(request, err);
    } else {
        respond_from_value(request, err);
    }
}

fn handle_request(repo_root: &PathBuf, port: u16, request: tiny_http::Request) {
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
            respond_todo_api_gated(request, &path, err);
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
            "/api/corpus-files" => {
                handle_corpus_files_post(repo_root, request);
                return;
            }
            "/api/archive-document" => {
                handle_archive_post(repo_root, request, archive_document);
                return;
            }
            "/api/archive-digest" => {
                handle_archive_post(repo_root, request, archive_digest);
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
                handle_todo_tasks_get(request);
                return;
            }
            "/api/todo-task" => {
                let params = parse_query(&url);
                let id = params.get("id").map(String::as_str).unwrap_or("");
                let value = todo_task::get_by_id(id);
                let (status, body) = todo_task_get_response_body(&value);
                respond_raw(request, status, body);
                return;
            }
            "/api/corpus-catalog" => {
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
                let value = get_corpus_catalog_latest_per_topic(repo_root);
                respond_from_value(request, value);
                return;
            }
            "/api/corpus-index" => {
                let value = get_corpus_index(repo_root);
                respond_from_value(request, value);
                return;
            }
            "/api/corpus-file" => {
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
                let value = get_corpus_file(repo_root, layer, common_path);
                respond_from_value(request, value);
                return;
            }
            "/api/corpus-asset" => {
                let params = parse_query(&url);
                let layer = params.get("layer").map(String::as_str).unwrap_or("");
                let base = params.get("base").map(String::as_str).unwrap_or("");
                let href = params.get("href").map(String::as_str).unwrap_or("");
                let value = get_corpus_asset(repo_root, layer, base, href);
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
            _ => {
                respond_json(request, 404, json!({ "error": "Not found" }));
                return;
            }
        }
    }

    respond_json(request, 405, json!({ "error": "Method not allowed" }));
}

fn handle_read_later_get(request: tiny_http::Request) {
    let entries = read_later::list_entries();
    let body = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string());
    respond_with_cors(request, 200, body);
}

fn handle_todo_tasks_get(request: tiny_http::Request) {
    let value = todo_task::list_all();
    if value.is_array() {
        let body = todo_tasks_list_response_body(&value);
        respond_with_cors(request, 200, body);
        return;
    }
    respond_read_later_from_value(request, value);
}

fn handle_read_later_post(mut request: tiny_http::Request) {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        respond_read_later_json(request, 400, json!({ "error": "Failed to read body" }));
        return;
    }
    let payload: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            respond_read_later_json(
                request,
                400,
                json!({ "error": format!("Invalid JSON: {e}") }),
            );
            return;
        }
    };
    let url = payload.get("url").and_then(|v| v.as_str());
    let title = payload.get("title").and_then(|v| v.as_str());
    let value = match url {
        Some(u) => read_later::create_entry(u, title),
        None => json!({ "error": "Missing url", "_status": 400 }),
    };
    respond_read_later_from_value(request, value);
}

fn handle_read_later_patch(mut request: tiny_http::Request, id: &str) {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        respond_read_later_json(request, 400, json!({ "error": "Failed to read body" }));
        return;
    }
    let payload: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            respond_read_later_json(
                request,
                400,
                json!({ "error": format!("Invalid JSON: {e}") }),
            );
            return;
        }
    };
    let read = payload.get("read").and_then(|v| v.as_bool()).unwrap_or(false);
    let value = read_later::mark_read(id, read);
    respond_read_later_from_value(request, value);
}

fn handle_read_later_delete(request: tiny_http::Request, id: &str) {
    let value = read_later::delete_entry(id);
    respond_read_later_from_value(request, value);
}

fn handle_corpus_files_post(repo_root: &PathBuf, mut request: tiny_http::Request) {
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
    let Some(ids_val) = payload.get("ids").and_then(|v| v.as_array()) else {
        respond_json(request, 400, json!({ "error": "Missing ids array" }));
        return;
    };
    let ids: Vec<String> = ids_val
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let value = get_corpus_files_by_ids(repo_root, &ids);
    if value.get("_status").is_some() {
        respond_from_value(request, value);
        return;
    }
    respond_json(request, 200, value);
}

fn handle_archive_post(
    repo_root: &PathBuf,
    mut request: tiny_http::Request,
    handler: fn(&Path, &Value) -> Value,
) {
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
    let value = handler(repo_root, &payload);
    respond_from_value(request, value);
}

fn read_json_body(request: &mut tiny_http::Request) -> Result<Value, Value> {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Err(json!({ "error": "Failed to read body" }));
    }
    serde_json::from_str(&body).map_err(|e| json!({ "error": format!("Invalid JSON: {e}") }))
}

fn handle_todo_task_post(
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

fn handle_todo_task_delete_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    todo_task::delete_master(master_task_id)
}

fn handle_todo_task_add_sub_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(title) = payload.get("title").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing title", "_status": 400 });
    };
    let content = payload.get("content").and_then(|v| v.as_str());
    todo_task::add_sub(master_task_id, title, content)
}

fn handle_todo_task_update_sub_payload(payload: &Value) -> Value {
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
    todo_task::update_sub_title(master_task_id, sub_task_id, title, content)
}

fn handle_todo_task_delete_sub_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(sub_task_id) = payload.get("sub_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing sub_task_id", "_status": 400 });
    };
    todo_task::delete_sub(master_task_id, sub_task_id)
}

fn handle_todo_task_complete_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let sub_task_id = payload.get("sub_task_id").and_then(|v| v.as_str());
    todo_task::complete_todo(master_task_id, sub_task_id)
}

fn handle_todo_task_set_status_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(status) = payload.get("status").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing status", "_status": 400 });
    };
    todo_task::set_master_status(master_task_id, status)
}

fn handle_todo_task_link_archive_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(sub_task_id) = payload.get("sub_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing sub_task_id", "_status": 400 });
    };
    let Some(archive_id) = payload.get("archive_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing archive_id", "_status": 400 });
    };
    todo_task::link_archive(master_task_id, sub_task_id, archive_id)
}

fn handle_todo_task_add_attachment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(file_name) = payload.get("file_name").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing file_name", "_status": 400 });
    };
    let Some(content) = payload.get("content").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing content", "_status": 400 });
    };
    todo_task::add_attachment(master_task_id, file_name, content)
}

fn handle_todo_task_list_attachments_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    todo_task::list_attachments(master_task_id)
}

fn handle_todo_task_get_attachment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(file_name) = payload.get("file_name").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing file_name", "_status": 400 });
    };
    todo_task::read_attachment(master_task_id, file_name)
}

fn handle_todo_task_update_attachment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(file_name) = payload.get("file_name").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing file_name", "_status": 400 });
    };
    let Some(content) = payload.get("content").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing content", "_status": 400 });
    };
    todo_task::save_attachment(master_task_id, file_name, content)
}

fn handle_todo_task_list_comments_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    todo_task::list_comments(master_task_id)
}

fn handle_todo_task_add_comment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(body) = payload.get("body").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing body", "_status": 400 });
    };
    todo_task::add_comment(master_task_id, body)
}

fn handle_todo_task_update_comment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(comment_id) = payload.get("comment_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing comment_id", "_status": 400 });
    };
    let Some(body) = payload.get("body").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing body", "_status": 400 });
    };
    todo_task::update_comment(master_task_id, comment_id, body)
}

fn handle_todo_task_delete_comment_payload(payload: &Value) -> Value {
    let Some(master_task_id) = payload.get("master_task_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing master_task_id", "_status": 400 });
    };
    let Some(comment_id) = payload.get("comment_id").and_then(|v| v.as_str()) else {
        return json!({ "error": "Missing comment_id", "_status": 400 });
    };
    todo_task::delete_comment(master_task_id, comment_id)
}

fn handle_todo_task_update_payload(payload: &Value) -> Value {
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
    if title.is_none() && todo_md.is_none() {
        return json!({ "error": "Missing title or todo_md", "_status": 400 });
    }

    todo_task::update_master_fields(master_task_id, title, todo_md)
}

fn handle_todo_task_create(mut request: tiny_http::Request) {
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
        None | Some(Value::Null) => "",
        Some(Value::String(s)) => s.as_str(),
        Some(_) => {
            respond_json(request, 400, json!({ "error": "Invalid todo_md" }));
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
            todo_task::create_master_with_subs_and_todo(title, Some(&refs), todo_md)
        }
        None => todo_task::create_master_with_subs_and_todo(title, None, todo_md),
    };
    respond_from_value(request, value);
}

fn parse_query(url: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(query) = url.split('?').nth(1) else {
        return out;
    };
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("").to_string();
        let value = parts.next().unwrap_or("").to_string();
        if !key.is_empty() {
            out.insert(
                urlencoding::decode(&key)
                    .map(|s| s.into_owned())
                    .unwrap_or(key),
                urlencoding::decode(&value)
                    .map(|s| s.into_owned())
                    .unwrap_or(value),
            );
        }
    }
    out
}

fn respond_from_value(request: tiny_http::Request, value: Value) {
    let (status, body) = map_value_to_response(value);
    respond_raw(request, status, body);
}

fn respond_json(request: tiny_http::Request, status: u16, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    respond_raw(request, status, body);
}

fn respond_read_later_from_value(request: tiny_http::Request, value: Value) {
    let (status, body) = map_value_to_response(value);
    respond_with_cors(request, status, body);
}

fn respond_read_later_json(request: tiny_http::Request, status: u16, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    respond_with_cors(request, status, body);
}

fn cors_headers() -> [Header; 3] {
    [
        Header::from_bytes(b"Access-Control-Allow-Origin", b"*").expect("cors origin"),
        Header::from_bytes(b"Access-Control-Allow-Methods", b"GET, POST, PATCH, OPTIONS")
            .expect("cors methods"),
        Header::from_bytes(b"Access-Control-Allow-Headers", b"Content-Type").expect("cors headers"),
    ]
}

fn respond_with_cors(request: tiny_http::Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    for header in cors_headers() {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

fn respond_with_cors_empty(request: tiny_http::Request, status: u16) {
    let mut response = Response::from_string("").with_status_code(StatusCode(status));
    for header in cors_headers() {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

fn respond_raw(request: tiny_http::Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

#[cfg(test)]
#[path = "../../unit-tests/services/local_http.rs"]
mod tests;
