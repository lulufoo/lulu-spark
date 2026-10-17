//! MCP tool catalog: business groups, factory, runtime assembly.

pub mod factory;
pub mod groups;
pub mod route_util;
pub mod runtime;

pub use factory::{build, group_for_migrated_api};
pub use runtime::{build_routes_for_channel, resolve_route};

use crate::mcp_host::ToolRoute;

/// Grouped catalog for Settings checkboxes. Not a slot table.
pub fn catalog_groups() -> Vec<(&'static str, Vec<ToolRoute>)> {
    vec![
        (groups::global::GROUP_ID, groups::global::catalog_snapshot_routes()),
        (groups::workspace::GROUP_ID, groups::workspace::catalog_snapshot_routes()),
        (groups::project::GROUP_ID, groups::project::catalog_snapshot_routes()),
        (groups::document::GROUP_ID, groups::document::catalog_snapshot_routes()),
        (groups::read_later::GROUP_ID, groups::read_later::catalog_snapshot_routes()),
    ]
}

#[cfg(test)]
#[path = "../../unit-tests/mcp_host/catalog.rs"]
mod tests;
