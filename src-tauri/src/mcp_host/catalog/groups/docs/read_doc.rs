//! MCP APIs: `get_doc_digest`, `get_doc_file` (desktop), `get_doc_content` (mobile).

use serde_json::{json, Value};

use super::schema::workspace_property;
use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

fn id_properties(extra: Value) -> Value {
    let mut props = workspace_property();
    props["id"] = json!({
        "type": "string",
        "description": "Document id from search or create_doc."
    });
    if let Some(extra) = extra.as_object() {
        for (key, value) in extra {
            props[key] = value.clone();
        }
    }
    props
}

pub fn build_digest(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "get_doc_digest",
        "Read one document's digest Markdown by id. workspace defaults to Default; pass the workspace the search hit came from.",
        object_schema(id_properties(json!({})), &["id"]),
        true,
        false,
        docs::get_digest,
    ))
}

/// Desktop channels only; mobile keeps `get_doc_content`.
pub fn build_file(channel: &str) -> Option<ToolRoute> {
    if channel == "mobile" {
        return None;
    }
    let props = id_properties(json!({
        "dest_dir": {
            "type": "string",
            "description": "Absolute directory to write the copy into. Must be a location you can read and write. Created if missing."
        }
    }));
    Some(route(
        "get_doc_file",
        "Copy one document's file into dest_dir and return the copy's path. The document itself never changes and its body is not returned. The copy is named <id>.md; if that name is taken it becomes <id>-v1.md, <id>-v2.md, and so on, and existing files are never overwritten. Edit the copy, then call update_doc with source_path set to it. workspace defaults to Default; pass the workspace the search hit came from. Result: {ok, id, path}. Unknown ids fail.",
        object_schema(props, &["id", "dest_dir"]),
        false,
        false,
        docs::export_file,
    ))
}

/// Mobile only; desktop channels use `get_doc_file`.
pub fn build_content(channel: &str) -> Option<ToolRoute> {
    if channel != "mobile" {
        return None;
    }
    Some(route(
        "get_doc_content",
        "Resolve a document id from search to the local absolute file path. workspace defaults to Default; pass the workspace the search hit came from. Does not return file body. Unknown ids fail.",
        object_schema(id_properties(json!({})), &["id"]),
        true,
        false,
        docs::get_path,
    ))
}
