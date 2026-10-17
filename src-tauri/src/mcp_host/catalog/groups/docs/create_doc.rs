//! MCP API: `create_doc`

use serde_json::{json, Value};

use super::schema::create_doc_properties;
use super::write_public::mcp_doc_write_public;
use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;
use crate::services::message_center;

pub(crate) fn produce_if_ok(result: &Value) {
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

fn after_write(result: Value) -> Value {
    produce_if_ok(&result);
    mcp_doc_write_public(result)
}

pub fn invoke_from_content(args: &Value) -> Value {
    after_write(docs::create_doc_content(args))
}

pub fn invoke_from_source(args: &Value) -> Value {
    after_write(docs::create_doc(args))
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if channel == "mobile" {
        return Some(route(
            "create_doc",
            "Create a document in a workspace and project from Markdown content. Send title and body; do not send source_path. workspace defaults to Default and project to inbox. This channel has no companion-image field; relative images will not display. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. digest_body must not contain relative images. Success is {ok, id, workspace, workspace_name}; digest_written or zh_written is true when that layer was written.",
            object_schema(create_doc_properties(false), &["content", "title", "digest"]),
            false,
            false,
            invoke_from_content,
        ));
    }
    Some(route(
        "create_doc",
        "Create a document in a workspace and project from a Markdown file by absolute source_path. workspace defaults to Default and project to inbox. The Host reads the file; never send document text. If the source Markdown uses relative images (![]() or <img src>), this same call MUST include asset_paths; omit asset_paths when there are none. Host copies only the listed files and does not scan the Markdown. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. digest_body must not contain relative images. Success is {ok, id, workspace, workspace_name}; digest_written, zh_written, or assets_copied is true when that layer was written.",
        object_schema(create_doc_properties(true), &["source_path", "title", "digest"]),
        false,
        false,
        invoke_from_source,
    ))
}
