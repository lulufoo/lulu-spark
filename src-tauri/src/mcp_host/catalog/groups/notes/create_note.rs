//! MCP API: `create_note`

use std::path::Path;

use serde_json::{json, Value};

use super::schema::create_note_properties;
use super::write_public::mcp_note_write_public;
use crate::mcp_host::catalog::route_util::{notes_repo_root, object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::message_center;
use crate::services::notes::{create_note, create_note_content};

pub(crate) fn produce_notes_if_ok(result: &Value) {
    if result.get("error").is_none() && result.get("ok") == Some(&json!(true)) {
        let mut params = serde_json::Map::new();
        if let Some(id) = result.get("id") {
            params.insert("archive_id".to_string(), id.clone());
        }
        let _ = message_center::produce(message_center::Envelope {
            business: "notes".to_string(),
            action: "create".to_string(),
            params: Value::Object(params),
        });
    }
}

fn invoke_after_write(args: &Value, write: fn(&Path, &Value) -> Value) -> Value {
    match notes_repo_root() {
        Ok(root) => {
            let result = write(&root, args);
            produce_notes_if_ok(&result);
            mcp_note_write_public(result)
        }
        Err(err) => err,
    }
}

pub fn available_in(_channel: &str) -> bool {
    true
}

pub fn invoke_from_content(args: &Value) -> Value {
    invoke_after_write(args, create_note_content)
}

pub fn invoke_from_source(args: &Value) -> Value {
    invoke_after_write(args, create_note)
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if !available_in(channel) {
        return None;
    }
    if channel == "mobile" {
        return Some(route(
            "create_note",
            "Create a note from Markdown content. Send title and body; do not send source_path. This channel has no companion-image field; relative images will not display. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. digest_body must not contain relative images. Success is {ok, id}; digest_written or zh_written is true when that layer was written.",
            object_schema(create_note_properties(false), &["content", "title", "digest"]),
            false,
            false,
            invoke_from_content,
        ));
    }
    Some(route(
        "create_note",
        "Create a note from a Markdown file by absolute source_path. The Host reads the file; never send document text. If the source Markdown uses relative images (![]() or <img src>), this same call MUST include asset_paths; omit asset_paths when there are none. Host copies only the listed files and does not scan the Markdown. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. digest_body must not contain relative images. Success is {ok, id}; digest_written, zh_written, or assets_copied is true when that layer was written.",
        object_schema(create_note_properties(true), &["source_path", "title", "digest"]),
        false,
        false,
        invoke_from_source,
    ))
}
