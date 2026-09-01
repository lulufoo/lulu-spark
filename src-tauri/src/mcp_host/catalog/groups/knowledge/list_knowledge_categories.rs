//! MCP API: `list_knowledge_categories`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::knowledge::list_knowledge_categories_value;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(_args: &Value) -> Value {
    list_knowledge_categories_value()
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "list_knowledge_categories",
        "List knowledge categories (id, name). Does not list repos or documents. Includes uncategorized.",
        object_schema(json!({}), &[]),
        true,
        false,
        invoke,
    ))
}
