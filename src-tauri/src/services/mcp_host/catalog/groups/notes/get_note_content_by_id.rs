//! MCP API: `get_note_content_by_id`

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
    if channel == "workbench" {
        return Some(route(
            "get_note_content_by_id",
            "Stage one note onto this Chat and return the staged document id (F1, F2, …). Input id is the archive entry id. Does not return file path or body. The user reads the file from the Stage list. Use get_note_digest_by_id for a digest.",
            HttpMethod::Post,
            "/api/note-path",
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
        ));
    }
    Some(route(
        "get_note_content_by_id",
        "Read one note's raw Markdown by archive entry id. Bodies longer than 10KB are truncated.",
        HttpMethod::Post,
        "/api/note-content",
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
