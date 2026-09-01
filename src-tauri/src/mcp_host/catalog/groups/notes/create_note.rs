//! MCP API: `create_note`

use serde_json::Value;

use super::schema::create_note_properties;
use crate::mcp_host::catalog::route_util::{notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::notes::{create_note, create_note_content};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke_from_content(args: &Value) -> Value {
    match notes_repo_root() {
        Ok(root) => create_note_content(&root, args),
        Err(err) => err,
    }
}

pub fn invoke_from_source(args: &Value) -> Value {
    match notes_repo_root() {
        Ok(root) => create_note(&root, args),
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
