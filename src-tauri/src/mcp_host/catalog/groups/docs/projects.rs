//! MCP APIs for projects: `list_projects`, `create_project`, `update_project`, `delete_project`.

use serde_json::json;

use super::schema::workspace_property;
use crate::mcp_host::catalog::route_util::{object_schema, route};
use crate::mcp_host::ToolRoute;
use crate::services::docs;

fn with(extra: serde_json::Value) -> serde_json::Value {
    let mut props = workspace_property();
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
        "List the projects of one workspace. workspace defaults to Default. Each project has a folder (the value other tools take as project), a title and a description.",
        object_schema(with(json!({})), &[]),
        true,
        false,
        docs::list_projects,
    ))
}

pub fn build_create(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "create_project",
        "Add a project to a workspace. workspace defaults to Default. Does not move existing documents.",
        object_schema(
            with(json!({
                "folder": { "type": "string", "description": "Optional directory name. Generated when omitted." },
                "title": { "type": "string", "description": "Display title." },
                "description": { "type": "string", "description": "Optional description." }
            })),
            &["title"],
        ),
        false,
        false,
        docs::create_project,
    ))
}

pub fn build_update(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "update_project",
        "Change a project's title and description. The folder is kept.",
        object_schema(
            with(json!({
                "project": { "type": "string", "description": "Project folder from list_projects." },
                "title": { "type": "string", "description": "Display title." },
                "description": { "type": "string", "description": "Optional description." }
            })),
            &["project", "title"],
        ),
        false,
        false,
        docs::update_project,
    ))
}

pub fn build_delete(_channel: &str) -> Option<ToolRoute> {
    Some(route(
        "delete_project",
        "Delete a project. The inbox project and a project that still holds documents are refused.",
        object_schema(
            with(json!({
                "project": { "type": "string", "description": "Project folder from list_projects." }
            })),
            &["project"],
        ),
        false,
        false,
        docs::delete_project,
    ))
}
