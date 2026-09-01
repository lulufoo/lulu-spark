//! MCP API: `list_notes_categories`

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
        "list_notes_categories",
        "List notes categories (id, title, description, folder). Does not list notes. Inbox is always present.",
        HttpMethod::Get,
        "/api/notes-categories",
        object_schema(json!({}), &[]),
        true,
        false,
    ))
}
