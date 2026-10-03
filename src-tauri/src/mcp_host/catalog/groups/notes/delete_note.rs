//! MCP API: `delete_note`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;

pub fn available_in(channel: &str) -> bool {
    channel == "spark"
}

pub fn invoke(args: &Value) -> Value {
    crate::services::entry_admin::delete_entry(args)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "delete_note",
        "Hard-delete one note by archive entry id (raw, digest, annotation, index). Irreversible. Exposed only on the workbench MCP channel (/mcp/spark); not available on cursor_ide or mobile.",
        object_schema(
            json!({
                "id": {
                    "type": "string",
                    "description": "Archive entry id (32-char hex)."
                }
            }),
            &["id"],
        ),
        false,
        false,
        invoke,
    ))
}
