//! Assemble enabled tool routes for one MCP channel via catalog factory.

use std::collections::HashSet;

use crate::services::mcp_host::ToolRoute;
use crate::services::settings::mcp_catalog::GroupedEnabledCatalog;

use super::factory;

pub fn build_routes_for_channel(
    _scene_slot: &str,
    channel: &str,
    enabled: &GroupedEnabledCatalog,
) -> Vec<ToolRoute> {
    let mut routes = Vec::new();
    let mut seen = HashSet::new();

    for (group, keys) in [("notes", &enabled.notes), ("todo", &enabled.todo)] {
        for key in keys {
            if !seen.insert(key.clone()) {
                continue;
            }
            if let Some(route) = factory::build(group, key, channel) {
                routes.push(route);
            }
        }
    }
    routes
}

pub fn resolve_route(_scene_slot: &str, channel: &str, tool: &str) -> Option<ToolRoute> {
    if let Some(group) = factory::group_for_migrated_api(tool) {
        return factory::build(group, tool, channel);
    }
    None
}
