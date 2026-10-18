//! MCP API: `update_doc`

use serde_json::{json, Value};

use super::super::schema::update_doc_properties;
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
            business: "docs".to_string(),
            action: "update".to_string(),
            params: Value::Object(params),
        });
    }
}

fn after_write(result: Value) -> Value {
    produce_if_ok(&result);
    mcp_doc_write_public(result)
}

pub fn invoke_from_content(args: &Value) -> Value {
    after_write(docs::update_doc_content(args))
}

pub fn invoke_from_source(args: &Value) -> Value {
    after_write(docs::update_doc(args))
}

pub fn build(channel: &str) -> Option<ToolRoute> {
    if channel == "mobile" {
        return Some(route(
            "update_doc",
            "Replace an existing document body by id. Send content; do not send source_path. The document stays in its workspace and project. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. Does not change title, translations, or location. Success is {ok, id, workspace_id, workspace_name, project_id, project_name}; digest_written is true when a digest is written.",
            object_schema(update_doc_properties(false), &["id", "content", "digest"]),
            false,
            false,
            invoke_from_content,
        ));
    }
    Some(route(
        "update_doc",
        "Replace an existing document body by id. Send an allow-listed Markdown source_path; the Host reads the file and never accepts document text. The document stays in its workspace and project. digest is required (auto|always|never). When a digest is written, pass digest_body on this same call. Does not change title, translations, or location. Success is {ok, id, workspace_id, workspace_name, project_id, project_name}; digest_written is true when a digest is written.",
        object_schema(update_doc_properties(true), &["id", "source_path", "digest"]),
        false,
        false,
        invoke_from_source,
    ))
}
