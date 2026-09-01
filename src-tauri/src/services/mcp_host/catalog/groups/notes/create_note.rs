//! MCP API: `create_note`

use super::schema::create_note_properties;
use crate::services::mcp_host::catalog::route_util::{object_schema, route};
use crate::services::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    if channel == "mobile" {
        return Some(route(
            "create_note",
            "Create a note from Markdown content. Send title and body; do not send source_path. Path is {project}/{theme}/{created_at}-{6-char}.md. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call.",
            HttpMethod::Post,
            "/api/create-note-content",
            object_schema(create_note_properties(false), &["content", "title", "digest"]),
            false,
            false,
        ));
    }
    Some(route(
        "create_note",
        "Create a note from an allow-listed Markdown file by absolute source_path. The Host reads the file; never send document text. Path is {project}/{theme}/{created_at}-{6-char}-{source filename}. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call.",
        HttpMethod::Post,
        "/api/create-note",
        object_schema(create_note_properties(true), &["source_path", "title", "digest"]),
        false,
        false,
    ))
}
