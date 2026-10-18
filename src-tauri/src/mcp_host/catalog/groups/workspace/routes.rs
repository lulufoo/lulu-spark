use serde_json::json;

use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

fn list_invoke(_args: &serde_json::Value) -> serde_json::Value {
    docs::list_workspaces()
}

pub fn build_list(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "list_workspaces",
        "List workspaces. Each has a workspace_id, a workspace_name and a view mode. The Default workspace always exists. Pass a workspace_id to the document and project tools.",
        object_schema(json!({}), &[]),
        true,
        false,
        list_invoke,
    ))
}

pub fn build_create(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "create_workspace",
        "Create a workspace. Returns its generated workspace_id. Every workspace starts with an empty inbox project.",
        object_schema(
            json!({
                "workspace_name": { "type": "string", "description": "Display name." },
                "view_mode": {
                    "type": "string",
                    "enum": ["directory", "timeline"],
                    "description": "Optional list layout. Defaults to directory."
                }
            }),
            &["workspace_name"],
        ),
        false,
        false,
        docs::create_workspace,
    ))
}

pub fn build_rename(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "rename_workspace",
        "Rename a workspace, including the Default one. The workspace_id never changes.",
        object_schema(
            json!({
                "workspace_id": { "type": "string", "description": "Workspace id from list_workspaces." },
                "workspace_name": { "type": "string", "description": "New display name." }
            }),
            &["workspace_id", "workspace_name"],
        ),
        false,
        false,
        docs::rename_workspace,
    ))
}
