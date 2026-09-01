//! MCP API: `create_notes_category`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{arg_str, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::notes::create_notes_category_value;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    let id = arg_str(args, "id").unwrap_or("");
    let title = arg_str(args, "title").unwrap_or("");
    let description = arg_str(args, "description").unwrap_or("");
    create_notes_category_value(id, title, description)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "create_notes_category",
        "Add a notes category. Id is generated as UTC+8 timestamp plus 6 random chars. Does not move existing notes.",
        object_schema(
            json!({
                "title": { "type": "string", "description": "Display title." },
                "description": { "type": "string", "description": "Optional description." }
            }),
            &["title"],
        ),
        false,
        false,
        invoke,
    ))
}
