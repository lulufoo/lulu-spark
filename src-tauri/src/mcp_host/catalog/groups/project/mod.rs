//! Project business group: `list_projects`, `create_project`, `update_project`.
//!
//! There is no delete, disable or enable tool: a project is disabled and enabled by a person in
//! Settings only.

mod routes;

use super::{
    registry_build, registry_contains, registry_routes_for_channel, registry_snapshot_routes,
    BuildFn, Registry,
};
use crate::mcp_host::ToolRoute;

pub const GROUP_ID: &str = "project";

const REGISTRY: Registry = &[
    ("list_projects", routes::build_list as BuildFn),
    ("create_project", routes::build_create as BuildFn),
    ("update_project", routes::build_update as BuildFn),
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
