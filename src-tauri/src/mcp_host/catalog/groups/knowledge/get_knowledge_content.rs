//! MCP API: `get_knowledge_content`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::knowledge::get_knowledge_path_by_id;

/// Mobile only; desktop channels use `get_knowledge_file`.
pub fn available_in(channel: &str) -> bool {
    channel == "mobile"
}

pub fn invoke(args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(|v| v.as_str()) else {
        return missing_field("id");
    };
    get_knowledge_path_by_id(id)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    let id_schema = object_schema(
        json!({
            "id": {
                "type": "string",
                "description": "Document id from search_document (category knowledge). Unknown ids fail."
            }
        }),
        &["id"],
    );
    Some(route(
        "get_knowledge_content",
        "Resolve a search_document knowledge document id to the local absolute file path. Does not return file body. Unknown ids fail.",
        id_schema,
        true,
        false,
        invoke,
    ))
}
