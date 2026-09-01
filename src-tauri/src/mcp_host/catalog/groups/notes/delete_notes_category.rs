//! MCP API: `delete_notes_category`

use serde_json::json;

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "delete_notes_category",
        "Remove a notes category from the allow-list. Does not delete or move existing notes. Inbox cannot be deleted.",
        HttpMethod::Post,
        "/api/notes-category-delete",
        object_schema(
            json!({
                "id": { "type": "string", "description": "Category id to remove from the allow-list." }
            }),
            &["id"],
        ),
        false,
        false,
    ))
}
