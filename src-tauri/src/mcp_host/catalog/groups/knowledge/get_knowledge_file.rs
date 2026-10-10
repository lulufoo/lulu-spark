//! MCP API: `get_knowledge_file`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{object_schema, require_str, route};
use crate::mcp_host::ToolRoute;
use crate::services::knowledge::get_knowledge_file;

/// Desktop channels only; mobile keeps `get_knowledge_content`.
pub fn available_in(channel: &str) -> bool {
    channel != "mobile"
}

pub fn invoke(args: &Value) -> Value {
    let id = match require_str(args, "id") {
        Ok(id) => id,
        Err(err) => return err,
    };
    let dest_dir = match require_str(args, "dest_dir") {
        Ok(dir) => dir,
        Err(err) => return err,
    };
    get_knowledge_file(id, dest_dir)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    let schema = object_schema(
        json!({
            "id": {
                "type": "string",
                "description": "Document id from search_document (category knowledge). Unknown ids fail."
            },
            "dest_dir": {
                "type": "string",
                "description": "Absolute directory to write the copy into. Must be a location you can read and write. Created if missing."
            }
        }),
        &["id", "dest_dir"],
    );
    Some(route(
        "get_knowledge_file",
        "Copy one knowledge document into dest_dir and return the copy's path. The document itself never changes and its body is not returned. The copy is named <id> plus the source file extension; if that name is taken it becomes <id>-v1, <id>-v2, and so on, and existing files are never overwritten. Result: {ok, id, path}. Unknown ids fail.",
        schema,
        false,
        false,
        invoke,
    ))
}
