//! MCP API: `get_note_digest`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::spark_read::get_note_digest;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(|v| v.as_str()) else {
        return missing_field("id");
    };
    match notes_repo_root() {
        Ok(root) => get_note_digest(&root, id),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_note_digest",
        "Read one note's digest Markdown by archive entry id.",
        object_schema(
            json!({
                "id": {
                    "type": "string",
                    "description": "Archive entry id (32-char hex)."
                }
            }),
            &["id"],
        ),
        true,
        false,
        invoke,
    ))
}
