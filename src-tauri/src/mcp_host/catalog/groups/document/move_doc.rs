//! MCP API: move an existing document to a project in a specified workspace.

use serde_json::{json, Value};

use crate::mcp_host::catalog::route_util::{object_schema, require_nonempty_str, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

fn invoke(args: &Value) -> Value {
    let Some(fields) = args.as_object() else {
        return json!({ "ok": false, "message": "Expected an object" });
    };
    if let Some(key) = fields
        .keys()
        .find(|key| !["id", "target_workspace_id", "project_id"].contains(&key.as_str()))
    {
        return json!({ "ok": false, "message": format!("Unsupported field: {key}") });
    }
    if let Err(error) = require_nonempty_str(args, "target_workspace_id") {
        return json!({ "ok": false, "message": error["error"] });
    }

    let result = docs::move_doc_to_project(args);
    if result.get("ok") == Some(&json!(true)) {
        let message = if result.get("moved") == Some(&json!(false)) {
            "Document is already in the requested project; no move was needed."
        } else {
            "Document moved to the requested project."
        };
        json!({ "ok": true, "message": message })
    } else {
        let message = result
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("Could not move document");
        json!({ "ok": false, "message": message })
    }
}

pub fn build(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "move_doc",
        "Move an existing document by id to the specified workspace and project. Obtain target_workspace_id from list_workspaces and project_id from list_projects for that workspace. The document id identifies its current workspace. An already correctly placed document stays in its folder. Body, attachments, translations, and digest move together; a conflicting filename is renamed. Returns {ok, message}; the message says when no move was needed.",
        object_schema(
            json!({
                "id": { "type": "string", "description": "Document id from search." },
                "target_workspace_id": { "type": "string", "description": "Destination workspace id from list_workspaces." },
                "project_id": { "type": "string", "description": "Destination project id from list_projects in the destination workspace." }
            }),
            &["id", "target_workspace_id", "project_id"],
        ),
        false,
        false,
        invoke,
    ))
}
