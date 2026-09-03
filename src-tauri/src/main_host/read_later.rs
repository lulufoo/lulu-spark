use serde_json::{json, Value};

use crate::services::message_center;
use crate::services::read_later;

use super::respond::{respond_read_later_from_value, respond_read_later_json};

fn produce_read_later_if_created(value: &Value) {
    if value.get("error").is_none() && value.get("_status") == Some(&json!(201)) {
        let _ = message_center::produce("read_later");
    }
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
    produce_read_later_if_created(&value);
    respond_read_later_from_value(request, value);
}
