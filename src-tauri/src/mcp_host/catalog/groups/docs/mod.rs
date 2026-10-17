//! Docs business group — workspaces, projects and the documents inside them.

mod create_doc;
mod delete_doc;
mod projects;
mod read_doc;
mod schema;
mod update_doc;
mod workspaces;
mod write_public;

use crate::mcp_host::ToolRoute;

#[cfg(test)]
pub(crate) use create_doc::{
    invoke_from_content as create_doc_from_content, invoke_from_source as create_doc_from_source,
    produce_if_ok as produce_create_doc_if_ok,
};
#[cfg(test)]
pub(crate) use update_doc::{
    invoke_from_content as update_doc_from_content, invoke_from_source as update_doc_from_source,
    produce_if_ok as produce_update_doc_if_ok,
};

pub const GROUP_ID: &str = "docs";

/// Channel used for Settings snapshot metadata (canonical paths/descriptions).
const SNAPSHOT_CHANNEL: &str = "cursor_ide";

type BuildFn = fn(&str) -> Option<ToolRoute>;

const REGISTRY: &[(&str, BuildFn)] = &[
    ("list_workspaces", workspaces::build_list as BuildFn),
    ("create_workspace", workspaces::build_create as BuildFn),
    ("rename_workspace", workspaces::build_rename as BuildFn),
    ("list_projects", projects::build_list as BuildFn),
    ("create_project", projects::build_create as BuildFn),
    ("update_project", projects::build_update as BuildFn),
    ("get_doc_digest", read_doc::build_digest as BuildFn),
    ("get_doc_content", read_doc::build_content as BuildFn),
    ("get_doc_file", read_doc::build_file as BuildFn),
    ("create_doc", create_doc::build as BuildFn),
    ("update_doc", update_doc::build as BuildFn),
    ("delete_doc", delete_doc::build as BuildFn),
];

pub fn contains(api: &str) -> bool {
    REGISTRY.iter().any(|(key, _)| *key == api)
}

pub fn build(api: &str, channel: &str) -> Option<ToolRoute> {
    REGISTRY
        .iter()
        .find(|(key, _)| *key == api)
        .and_then(|(_, build)| build(channel))
}

/// Settings / slot table metadata without channel overlays (except spark-only tools).
pub fn catalog_snapshot_routes() -> Vec<ToolRoute> {
    REGISTRY
        .iter()
        .map(|(key, build_fn)| {
            build_fn(SNAPSHOT_CHANNEL)
                .or_else(|| build_fn("spark"))
                .or_else(|| build_fn("mobile"))
                .unwrap_or_else(|| panic!("catalog snapshot missing for {key}"))
        })
        .collect()
}

/// Enabled tool routes for one MCP channel.
pub fn routes_for_channel(channel: &str) -> Vec<ToolRoute> {
    REGISTRY
        .iter()
        .filter_map(|(_, build_fn)| build_fn(channel))
        .collect()
}
