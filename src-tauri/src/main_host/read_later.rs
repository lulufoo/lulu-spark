use serde_json::{json, Value};

use crate::services::read_later;

use super::respond::{
    respond_read_later_from_value, respond_read_later_json, respond_with_cors,
};

pub(super) fn handle_read_later_get(request: tiny_http::Request) {
    let entries = read_later::list_entries();
    let body = serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string());
    respond_with_cors(request, 200, body);
}
pub(super) fn handle_read_later_post(mut request: tiny_http::Request) {
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

pub(super) fn handle_read_later_patch(mut request: tiny_http::Request, id: &str) {
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

pub(super) fn handle_read_later_delete(request: tiny_http::Request, id: &str) {
    let value = read_later::delete_entry(id);
    respond_read_later_from_value(request, value);
}
