//! MCP API: `delete_doc` (spark channel only).

use serde_json::json;

use super::schema::workspace_property;
use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

pub fn build(channel: &str) -> Option<ToolRoute> {
    if channel != "spark" {
        return None;
    }
    let mut props = workspace_property();
    props["id"] = json!({ "type": "string", "description": "Document id from search or create_doc." });
    Some(route(
        "delete_doc",
        "Hard-delete one document by id (raw, digest, annotation, index). workspace defaults to Default. Irreversible. Exposed only on the spark MCP channel (/mcp/spark); not available on cursor_ide or mobile.",
        object_schema(props, &["id"]),
        false,
        false,
        docs::delete_doc,
    ))
}
