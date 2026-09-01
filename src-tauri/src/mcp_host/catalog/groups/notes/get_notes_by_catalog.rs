//! MCP API: `get_notes_by_catalog`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::workbench_read::list_notes_by_catalog;

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke(args: &Value) -> Value {
    let Some(catalog) = args.get("catalog").and_then(|v| v.as_str()) else {
        return missing_field("catalog");
    };
    match notes_repo_root() {
        Ok(root) => list_notes_by_catalog(&root, catalog),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    Some(route(
        "get_notes_by_catalog",
        "List every note id in one catalog. Catalog is the project name (inbox, ai). Returns ids only, newest first.",
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
        invoke,
    ))
}
