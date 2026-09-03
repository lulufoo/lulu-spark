//! MCP API: `create_note`

use serde_json::{json, Value};

use super::schema::create_note_properties;
use crate::mcp_host::catalog::route_util::{notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::message_center;
use crate::services::notes::{create_note, create_note_content};

fn produce_notes_if_ok(result: &Value) {
    if result.get("error").is_none() && result.get("ok") == Some(&json!(true)) {
        let _ = message_center::produce("notes");
    }
}

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke_from_content(args: &Value) -> Value {
    match notes_repo_root() {
        Ok(root) => {
            let result = create_note_content(&root, args);
            produce_notes_if_ok(&result);
            result
        }
        Err(err) => err,
    }
}

pub fn invoke_from_source(args: &Value) -> Value {
    match notes_repo_root() {
        Ok(root) => {
            let result = create_note(&root, args);
            produce_notes_if_ok(&result);
            result
        }
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    if channel == "mobile" {
        return Some(route(
            "create_note",
            "Create a note from Markdown content. Send title and body; do not send source_path. Path is {project}/{theme}/{created_at}-{6-char}. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call.",
            object_schema(create_note_properties(false), &["content", "title", "digest"]),
            false,
            false,
            invoke_from_content,
        ));
    }
    Some(route(
        "create_note",
        "Create a note from an allow-listed Markdown file by absolute source_path. The Host reads the file; never send document text. Path is {project}/{theme}/{created_at}-{6-char}-{source filename}. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call.",
        object_schema(create_note_properties(true), &["source_path", "title", "digest"]),
        false,
        false,
        invoke_from_source,
    ))
}
