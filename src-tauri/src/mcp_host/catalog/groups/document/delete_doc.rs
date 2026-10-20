//! MCP API: `delete_doc` (spark channel only).

use serde_json::json;

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

pub fn build(channel: &str) -> Option<ToolRoute> {
    if channel != "spark" {
        return None;
    }
    Some(route(
        "delete_doc",
        "Hard-delete one document by id (raw, digest, annotation, index). The document id already names its workspace; do not send workspace_id. Irreversible. Success is {ok, id, removed}; removed counts the files deleted. Exposed only on the spark MCP channel (/mcp/spark); not available on cursor_ide or mobile.",
        object_schema(
            json!({ "id": { "type": "string", "description": "Document id from search or create_doc." } }),
            &["id"],
        ),
        false,
        false,
        docs::delete_doc,
    ))
}
