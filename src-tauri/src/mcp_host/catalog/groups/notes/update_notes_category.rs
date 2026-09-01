//! MCP API: `update_notes_category`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{arg_str, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::notes::update_notes_category_value;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    let id = arg_str(args, "id").unwrap_or("");
    let title = arg_str(args, "title").unwrap_or("");
    let description = arg_str(args, "description").unwrap_or("");
    update_notes_category_value(id, title, description)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "update_notes_category",
        "Change a notes category title or description. Does not move existing notes or change the id.",
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
        invoke,
    ))
}
