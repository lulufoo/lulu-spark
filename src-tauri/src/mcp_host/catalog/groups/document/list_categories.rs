//! MCP API: `list_doc_categories`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

fn invoke(_args: &Value) -> Value {
    match docs::load_doc_categories() {
        Ok(items) => {
            let categories: Vec<Value> = items
                .into_iter()
                .filter(|c| c.exposed)
                .map(|c| json!({ "id": c.id, "label": c.label }))
                .collect();
            json!({ "ok": true, "categories": categories })
        }
        Err(e) => json!({ "error": e, "_status": 500 }),
    }
}

pub fn build(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "list_doc_categories",
        "List the document classification ids that create_doc.source_type accepts. Call this before create_doc. Returns {ok, categories: [{id, label}]}.",
        object_schema(json!({}), &[]),
        true,
        false,
        invoke,
    ))
}
