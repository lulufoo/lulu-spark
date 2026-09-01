use std::collections::HashMap;

use serde_json::{json, Value};
use tiny_http::{Header, Response, StatusCode};

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
pub(super) fn read_json_body(request: &mut tiny_http::Request) -> Result<Value, Value> {
    let mut body = String::new();
    if request.as_reader().read_to_string(&mut body).is_err() {
        return Err(json!({ "error": "Failed to read body" }));
    }
    serde_json::from_str(&body).map_err(|e| json!({ "error": format!("Invalid JSON: {e}") }))
}
pub(super) fn parse_query(url: &str) -> HashMap<String, String> {
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

pub(super) fn respond_from_value(request: tiny_http::Request, value: Value) {
    let (status, body) = map_value_to_response(value);
    respond_raw(request, status, body);
}

pub(super) fn respond_json(request: tiny_http::Request, status: u16, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    respond_raw(request, status, body);
}

pub(super) fn respond_read_later_from_value(request: tiny_http::Request, value: Value) {
    let (status, body) = map_value_to_response(value);
    respond_with_cors(request, status, body);
}

pub(super) fn respond_read_later_json(request: tiny_http::Request, status: u16, value: Value) {
    let body = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
    respond_with_cors(request, status, body);
}

pub(super) fn cors_headers() -> [Header; 3] {
    [
        Header::from_bytes(b"Access-Control-Allow-Origin", b"*").expect("cors origin"),
        Header::from_bytes(b"Access-Control-Allow-Methods", b"GET, POST, PATCH, OPTIONS")
            .expect("cors methods"),
        Header::from_bytes(b"Access-Control-Allow-Headers", b"Content-Type").expect("cors headers"),
    ]
}

pub(super) fn respond_with_cors(request: tiny_http::Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    for header in cors_headers() {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

pub(super) fn respond_with_cors_empty(request: tiny_http::Request, status: u16) {
    let mut response = Response::from_string("").with_status_code(StatusCode(status));
    for header in cors_headers() {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}

pub(super) fn respond_raw(request: tiny_http::Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    let _ = request.respond(response);
}
