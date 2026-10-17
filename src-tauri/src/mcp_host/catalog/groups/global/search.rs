//! MCP API: `search`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

pub fn invoke(args: &Value) -> Value {
    docs::search(args)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    let follow_up = if channel == "mobile" {
        "Then call get_doc_content with the id and workspace."
    } else {
        "Then call get_doc_file with the id and workspace."
    };
    Some(route(
        "search",
        &format!(
            "Search Lulu Spark's own library: the documents of every workspace, or of one when workspace is given. Returns id, title, snippet, workspace_name, workspace and project per hit. To link a hit in chat write [workspace_name][U+FF1A][title](spark-doc:id), where U+FF1A is the full-width colon. Does not return file path or body. {follow_up}"
        ),
        object_schema(
            json!({
                "q": {
                    "type": "string",
                    "description": "Search query."
                },
                "workspace": {
                    "type": "string",
                    "description": "Optional workspace id from list_workspaces. Searches every workspace when omitted."
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
