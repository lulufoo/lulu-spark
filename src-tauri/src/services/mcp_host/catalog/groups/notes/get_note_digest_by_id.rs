//! MCP API: `get_note_digest_by_id`

use serde_json::json;

use crate::services::mcp_host::catalog::route_util::{object_schema, route};
use crate::services::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_note_digest_by_id",
        "Read one note's digest Markdown by archive entry id.",
        HttpMethod::Post,
        "/api/note-digest",
        object_schema(
            json!({
                "id": {
                    "type": "string",
                    "description": "Archive entry id (32-char hex)."
                }
            }),
            &["id"],
        ),
        true,
        false,
    ))
}
