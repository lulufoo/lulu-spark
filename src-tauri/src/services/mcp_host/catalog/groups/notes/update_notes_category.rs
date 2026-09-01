//! MCP API: `update_notes_category`

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
        "update_notes_category",
        "Change a notes category title or description. Does not move existing notes or change the id.",
        HttpMethod::Post,
        "/api/notes-category-update",
        object_schema(
            json!({
                "id": { "type": "string", "description": "Category id. Cannot be renamed." },
                "title": { "type": "string", "description": "Display title." },
                "description": { "type": "string", "description": "Optional description." }
            }),
            &["id", "title"],
        ),
        false,
        false,
    ))
}
