//! MCP API: `search_notes`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::workbench_read::search_notes;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    let Some(q) = args.get("q").and_then(|v| v.as_str()) else {
        return missing_field("q");
    };
    let catalog = args.get("catalog").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| {
        v.as_u64()
            .map(|n| n as u32)
            .or_else(|| v.as_i64().and_then(|n| u32::try_from(n).ok()))
    });
    match notes_repo_root() {
        Ok(root) => search_notes(&root, q, catalog, limit),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "search_notes",
        "Search notes by text. Searches raw bodies only. Returns notes with match snippets. Does not return digest or raw bodies. Optional catalog limits to one project. Default limit 5.",
        object_schema(
            json!({
                "q": {
                    "type": "string",
                    "description": "Search query."
                },
                "catalog": {
                    "type": "string",
                    "description": "Optional project catalog name, such as inbox."
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional max notes to return. Default 5, max 50."
                }
            }),
            &["q"],
        ),
        true,
        false,
        invoke,
    ))
}
