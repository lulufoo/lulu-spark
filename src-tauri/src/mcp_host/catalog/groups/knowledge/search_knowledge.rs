//! MCP API: `search_knowledge`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::knowledge::search_knowledge_mcp;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    let Some(q) = args.get("q").and_then(|v| v.as_str()) else {
        return missing_field("q");
    };
    let limit = args.get("limit").and_then(|v| {
        v.as_u64()
            .map(|n| n as u32)
            .or_else(|| v.as_i64().and_then(|n| u32::try_from(n).ok()))
    });
    match notes_repo_root() {
        Ok(root) => search_knowledge_mcp(&root, q, limit),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "search_knowledge",
        "Search knowledge documents. Returns document id, title, and snippet. Does not return file path or body. Call get_knowledge_content with an id from this list.",
        object_schema(
            json!({
                "q": {
                    "type": "string",
                    "description": "Search query."
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional max hits. Default 10, max 50."
                }
            }),
            &["q"],
        ),
        true,
        false,
        invoke,
    ))
}
