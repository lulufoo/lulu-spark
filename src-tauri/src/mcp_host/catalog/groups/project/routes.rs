use serde_json::json;

use super::super::schema::workspace_id_property;
use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

fn with(extra: serde_json::Value) -> serde_json::Value {
    let mut props = workspace_id_property();
    if let Some(extra) = extra.as_object() {
        for (key, value) in extra {
            props[key] = value.clone();
        }
    }
    props
}

pub fn build_list(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "list_projects",
        "List the projects of one workspace. workspace_id defaults to Default. Each project has a project_id (the value other tools take as project_id), a project_name, is_default, a description, and doc_count (the number of documents in it).",
        object_schema(with(json!({})), &[]),
        true,
        false,
        docs::list_projects,
    ))
}

pub fn build_create(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "create_project",
        "Add a project to a workspace. workspace_id defaults to Default. Returns its generated project_id. Does not move existing documents.",
        object_schema(
            with(json!({
                "project_name": { "type": "string", "description": "Display name." },
                "description": { "type": "string", "description": "Optional description." }
            })),
            &["project_name"],
        ),
        false,
        false,
        docs::create_project,
    ))
}

pub fn build_update(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "update_project",
        "Change a project's name and description. The project_id is kept.",
        object_schema(
            with(json!({
                "project_id": { "type": "string", "description": "Project id from list_projects." },
                "project_name": { "type": "string", "description": "Display name." },
                "description": { "type": "string", "description": "Optional description." }
            })),
            &["project_id", "project_name"],
        ),
        false,
        false,
        docs::update_project,
    ))
}
