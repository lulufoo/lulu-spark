//! MCP API: `update_note`

use std::path::Path;

use serde_json::{json, Value};

use super::schema::update_note_properties;
use crate::mcp_host::catalog::route_util::{notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::message_center;
use crate::services::notes::{update_note, update_note_content};

pub(crate) fn produce_notes_if_ok(result: &Value) {
    if result.get("error").is_none() && result.get("ok") == Some(&json!(true)) {
        let mut params = serde_json::Map::new();
        if let Some(id) = result.get("id") {
            params.insert("id".to_string(), id.clone());
        }
        if let Some(path) = result.get("common_path") {
            params.insert("common_path".to_string(), path.clone());
        }
        let _ = message_center::produce(message_center::Envelope {
            business: "notes".to_string(),
            action: "update".to_string(),
            params: Value::Object(params),
        });
    }
}

fn invoke_after_write(args: &Value, write: fn(&Path, &Value) -> Value) -> Value {
    match notes_repo_root() {
        Ok(root) => {
            let result = write(&root, args);
            produce_notes_if_ok(&result);
            result
        }
        Err(err) => err,
    }
}

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke_from_content(args: &Value) -> Value {
    invoke_after_write(args, update_note_content)
}

pub fn invoke_from_source(args: &Value) -> Value {
    invoke_after_write(args, update_note)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    if channel == "mobile" {
        return Some(route(
            "update_note",
            "Replace an existing note body by archive entry id. Send content; do not send source_path. Keeps the existing path. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. Does not change title, translations, or folder.",
            object_schema(update_note_properties(false), &["id", "content", "digest"]),
            false,
            false,
            invoke_from_content,
        ));
    }
    Some(route(
        "update_note",
        "Replace an existing note body by archive entry id. Send an allow-listed Markdown source_path; the Host reads the file and never accepts document text. Destination is the existing common_path for id. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. Does not change title, translations, or folder.",
        object_schema(update_note_properties(true), &["id", "source_path", "digest"]),
        false,
        false,
        invoke_from_source,
    ))
}