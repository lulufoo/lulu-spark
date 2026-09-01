//! MCP API: `delete_notes_category`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{arg_str, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::notes::delete_notes_category_value;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    delete_notes_category_value(arg_str(args, "id").unwrap_or(""))
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "delete_notes_category",
        "Remove a notes category from the allow-list. Does not delete or move existing notes. Inbox cannot be deleted.",
        object_schema(
            json!({
                "id": { "type": "string", "description": "Category id to remove from the allow-list." }
            }),
            &["id"],
        ),
        false,
        false,
        invoke,
    ))
}
