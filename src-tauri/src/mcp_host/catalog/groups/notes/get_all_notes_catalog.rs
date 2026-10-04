//! MCP API: `get_all_notes_catalog`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::spark_read::list_all_notes_catalogs;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(_args: &Value) -> Value {
    match notes_repo_root() {
        Ok(root) => list_all_notes_catalogs(&root),
        Err(err) => err,
    }
}

pub fn build(_channel: &str) -> Option<ToolRoute> {
    if !available_in(_channel) {
        return None;
    }
    Some(route(
        "get_all_notes_catalog",
        "List every note catalog (project name) with the newest note_id and created_at. Does not return digest or raw bodies.",
        object_schema(json!({}), &[]),
        true,
        false,
        invoke,
    ))
}
