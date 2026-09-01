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
        ("notes", groups::notes::catalog_snapshot_routes()),
        ("todo", groups::todo::catalog_snapshot_routes()),
        ("knowledge", groups::knowledge::catalog_snapshot_routes()),
        ("global", groups::global::catalog_snapshot_routes()),
    ]
}

#[cfg(test)]
#[path = "../../unit-tests/mcp_host/catalog.rs"]
mod tests;
