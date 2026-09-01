//! MCP API: `search_notes`

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
        "search_notes",
        "Search notes by text. Searches raw bodies only. Returns notes with match snippets. Does not return digest or raw bodies. Optional catalog limits to one project. Default limit 5.",
        HttpMethod::Post,
        "/api/notes-search",
        object_schema(
            json!({
                "q": {
                    "type": "string",
                    "description": "Search query."
                },
                "catalog": {
                    "type": "string",
                    "description": "Optional project catalog name, such as inbox."
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional max notes to return. Default 5, max 50."
                }
            }),
            &["q"],
        ),
        true,
        false,
    ))
}
