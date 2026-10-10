//! MCP API: `get_read_later`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::read_later::get_entry;

pub fn available_in(channel: &str) -> bool {
    channel == "spark"
}

pub fn invoke(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(|v| v.as_str()) else {
        return missing_field("id");
    };
    get_entry(id)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_read_later",
        "Return one read-later entry by id: id, url, and read. Does not return a file or path. Input id is the 32-char hex id from [title](read-later:<id>).",
        object_schema(
            json!({
                "id": {
                    "type": "string",
                    "description": "Read Later entry id (32-char hex)."
                }
            }),
            &["id"],
        ),
        true,
        false,
        invoke,
    ))
}
