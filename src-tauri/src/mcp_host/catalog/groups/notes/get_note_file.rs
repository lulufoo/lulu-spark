//! MCP API: `get_note_file`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{notes_repo_root, object_schema, require_str, route};
use crate::mcp_host::ToolRoute;
use crate::services::notes::get_note_file;

/// Desktop channels only; mobile keeps `get_note_content`.
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
    match notes_repo_root() {
        Ok(root) => get_note_file(&root, id, dest_dir),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    let schema = object_schema(
        json!({
            "id": {
                "type": "string",
                "description": "Document id from search_document (category notes). Archive entry id, 32-char hex."
            },
            "dest_dir": {
                "type": "string",
                "description": "Absolute directory to write the copy into. Must be a location you can read and write. Created if missing."
            }
        }),
        &["id", "dest_dir"],
    );
    Some(route(
        "get_note_file",
        "Copy one note's file into dest_dir and return the copy's path. The note itself never changes and its body is not returned. The copy is named <id>.md; if that name is taken it becomes <id>-v1.md, <id>-v2.md, and so on, and existing files are never overwritten. Edit the copy, then call update_note with source_path set to it. Result: {ok, id, path}. Unknown ids fail.",
        schema,
        false,
        false,
        invoke,
    ))
}
