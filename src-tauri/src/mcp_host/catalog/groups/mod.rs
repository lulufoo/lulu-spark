//! Business groups registered with the catalog factory.
//!
//! Five groups, in the order Settings shows them: Global, Workspace, Project, Document,
//! Read Later. Each tool belongs to exactly one group.

pub mod document;
pub mod global;
pub mod project;
pub mod read_later;
pub(crate) mod schema;
pub mod workspace;

use crate::mcp_host::ToolRoute;

pub const ALL_GROUP_IDS: &[&str] = &[
    global::GROUP_ID,
    workspace::GROUP_ID,
    project::GROUP_ID,
    document::GROUP_ID,
    read_later::GROUP_ID,
];

/// Channel used for Settings snapshot metadata (canonical paths/descriptions).
const SNAPSHOT_CHANNEL: &str = "cursor_ide";

pub(crate) type BuildFn = fn(&str) -> Option<ToolRoute>;
pub(crate) type Registry = &'static [(&'static str, BuildFn)];

pub(crate) fn registry_contains(registry: Registry, api: &str) -> bool {
    registry.iter().any(|(key, _)| *key == api)
}

pub(crate) fn registry_build(registry: Registry, api: &str, channel: &str) -> Option<ToolRoute> {
    registry
        .iter()
        .find(|(key, _)| *key == api)
        .and_then(|(_, build)| build(channel))
}

/// Settings / slot table metadata without channel overlays (except spark-only tools).
pub(crate) fn registry_snapshot_routes(registry: Registry) -> Vec<ToolRoute> {
    registry
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
pub(crate) fn registry_routes_for_channel(registry: Registry, channel: &str) -> Vec<ToolRoute> {
    registry
        .iter()
        .filter_map(|(_, build_fn)| build_fn(channel))
        .collect()
}
