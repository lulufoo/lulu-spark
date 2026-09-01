//! MCP API: `list_notes_categories`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::notes::list_notes_categories_value;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(_args: &Value) -> Value {
    list_notes_categories_value()
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "list_notes_categories",
        "List notes categories (id, title, description, folder). Does not list notes. Inbox is always present.",
        object_schema(json!({}), &[]),
        true,
        false,
        invoke,
    ))
}
