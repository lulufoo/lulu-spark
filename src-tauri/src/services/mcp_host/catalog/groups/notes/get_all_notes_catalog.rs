//! MCP API: `get_all_notes_catalog`

use serde_json::json;

use crate::services::mcp_host::catalog::route_util::{object_schema, route};
use crate::services::mcp_host::{HttpMethod, ToolRoute};

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn build(_channel: &str) -> Option<ToolRoute> {
    if !available_in(_channel) {
        return None;
    }
    Some(route(
        "get_all_notes_catalog",
        "List every note catalog (project name) with the newest note_id and created_at. Does not return digest or raw bodies.",
        HttpMethod::Get,
        "/api/notes-catalogs",
        object_schema(json!({}), &[]),
        true,
        false,
    ))
}
