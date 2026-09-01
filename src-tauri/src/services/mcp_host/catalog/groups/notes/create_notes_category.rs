//! MCP API: `create_notes_category`

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
        "create_notes_category",
        "Add a notes category. Id is generated as UTC+8 timestamp plus 6 random chars. Does not move existing notes.",
        HttpMethod::Post,
        "/api/notes-category-create",
        object_schema(
            json!({
                "title": { "type": "string", "description": "Display title." },
                "description": { "type": "string", "description": "Optional description." }
            }),
            &["title"],
        ),
        false,
        false,
    ))
}
