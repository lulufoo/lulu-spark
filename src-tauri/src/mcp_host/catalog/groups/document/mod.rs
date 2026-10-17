//! Document business group: read, create, update and delete the documents of a workspace.

mod create_doc;
mod delete_doc;
mod list_categories;
mod read_doc;
mod update_doc;
mod write_public;

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

use super::{
    registry_build, registry_contains, registry_routes_for_channel, registry_snapshot_routes,
    BuildFn, Registry,
};
use crate::mcp_host::ToolRoute;

pub const GROUP_ID: &str = "document";

const REGISTRY: Registry = &[
    ("get_doc_digest", read_doc::build_digest as BuildFn),
    ("get_doc_content", read_doc::build_content as BuildFn),
    ("get_doc_file", read_doc::build_file as BuildFn),
    ("list_doc_categories", list_categories::build as BuildFn),
    ("create_doc", create_doc::build as BuildFn),
    ("update_doc", update_doc::build as BuildFn),
    ("delete_doc", delete_doc::build as BuildFn),
];

pub fn contains(api: &str) -> bool {
    registry_contains(REGISTRY, api)
}

pub fn build(api: &str, channel: &str) -> Option<ToolRoute> {
    registry_build(REGISTRY, api, channel)
}

pub fn catalog_snapshot_routes() -> Vec<ToolRoute> {
    registry_snapshot_routes(REGISTRY)
}

pub fn routes_for_channel(channel: &str) -> Vec<ToolRoute> {
    registry_routes_for_channel(REGISTRY, channel)
}
