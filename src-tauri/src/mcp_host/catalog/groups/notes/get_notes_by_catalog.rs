//! MCP API: `get_notes_by_catalog`

use serde_json::json;

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_notes_by_catalog",
        "List every note id in one catalog. Catalog is the project name (inbox, ai). Returns ids only, newest first.",
        HttpMethod::Post,
        "/api/notes-by-catalog",
        object_schema(
            json!({
                "catalog": {
                    "type": "string",
                    "description": "Project catalog name, such as inbox."
                }
            }),
            &["catalog"],
        ),
        true,
        false,
    ))
}
