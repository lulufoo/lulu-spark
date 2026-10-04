//! MCP API: `get_latest_digest_per_catalog`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::spark_read::get_latest_digest_per_catalog;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(_args: &Value) -> Value {
    match notes_repo_root() {
        Ok(root) => get_latest_digest_per_catalog(&root),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_latest_digest_per_catalog",
        "Return the newest digest Markdown for every catalog in one call. Each item is catalog, note_id, created_at, and content. Catalogs with no digest are omitted.",
        object_schema(json!({}), &[]),
        true,
        false,
        invoke,
    ))
}
