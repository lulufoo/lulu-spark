//! MCP API: `search_document`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::knowledge::search_document_mcp;

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
        Ok(root) => search_document_mcp(&root, q, limit),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    let follow_up = if channel == "mobile" {
        "For category notes, call get_note_content. For category knowledge, call get_knowledge_content."
    } else {
        "For category notes, call get_note_file. For category knowledge, call get_knowledge_file."
    };
    Some(route(
        "search_document",
        &format!(
            "Search notes and knowledge together. Returns id, title, snippet, and category (notes or knowledge). Does not return file path or body. {follow_up}"
        ),
        object_schema(
            json!({
                "q": {
                    "type": "string",
                    "description": "Search query."
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional max hits per source. Default 10, max 50."
                }
            }),
            &["q"],
        ),
        true,
        false,
        invoke,
    ))
}
