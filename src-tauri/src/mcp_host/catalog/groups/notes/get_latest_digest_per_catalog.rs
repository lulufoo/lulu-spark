//! MCP API: `get_latest_digest_per_catalog`

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
        "get_latest_digest_per_catalog",
        "Return the newest digest Markdown for every catalog in one call. Each item is catalog, note_id, created_at, and content. Catalogs with no digest are omitted.",
        HttpMethod::Get,
        "/api/notes-latest-digests",
        object_schema(json!({}), &[]),
        true,
        false,
    ))
}
