//! MCP API: `get_note_content`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::spark_read::get_note_path_by_id;

/// Mobile only; desktop channels use `get_note_file`.
pub fn available_in(channel: &str) -> bool {
    channel == "mobile"
}

pub fn invoke(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(|v| v.as_str()) else {
        return missing_field("id");
    };
    match notes_repo_root() {
        Ok(root) => get_note_path_by_id(&root, id),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    let id_schema = object_schema(
        json!({
            "id": {
                "type": "string",
                "description": "Document id from search_document (category notes). Archive entry id, 32-char hex."
            }
        }),
        &["id"],
    );
    Some(route(
        "get_note_content",
        "Resolve a search_document notes document id to the local absolute file path. Does not return file body. Unknown ids fail.",
        id_schema,
        true,
        false,
        invoke,
    ))
}
