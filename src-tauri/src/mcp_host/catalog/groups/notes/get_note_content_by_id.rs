//! MCP API: `get_note_content_by_id`

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{missing_field, notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::workbench_read::{get_note_content_by_id, get_note_path_by_id};

pub fn available_in(_channel: &str) -> bool {
    true
}

fn require_id(args: &Value) -> Result<&str, Value> {
    args.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| missing_field("id"))
}

pub fn invoke_path(args: &Value) -> Value {
    let id = match require_id(args) {
        Ok(id) => id,
        Err(err) => return err,
    };
    match notes_repo_root() {
        Ok(root) => get_note_path_by_id(&root, id),
        Err(err) => err,
    }
}

pub fn invoke_content(args: &Value) -> Value {
    let id = match require_id(args) {
        Ok(id) => id,
        Err(err) => return err,
    };
    match notes_repo_root() {
        Ok(root) => get_note_content_by_id(&root, id),
        Err(err) => err,
    }
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    let id_schema = object_schema(
        json!({
            "id": {
                "type": "string",
                "description": "Archive entry id (32-char hex)."
            }
        }),
        &["id"],
    );
    if channel == "workbench" {
        return Some(route(
            "get_note_content_by_id",
            "Stage one note onto this Chat and return the staged document id (F1, F2, …). Input id is the archive entry id. Does not return file path or body. The user reads the file from the Stage list. Use get_note_digest_by_id for a digest.",
            id_schema,
            true,
            false,
            invoke_path,
        ));
    }
    Some(route(
        "get_note_content_by_id",
        "Read one note's raw Markdown by archive entry id. Bodies longer than 10KB are truncated.",
        id_schema,
        true,
        false,
        invoke_content,
    ))
}
