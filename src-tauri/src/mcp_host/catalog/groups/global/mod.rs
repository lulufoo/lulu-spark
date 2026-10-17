//! Global business group — APIs that span more than one domain.

mod search;

use super::{
    registry_build, registry_contains, registry_routes_for_channel, registry_snapshot_routes,
    BuildFn, Registry,
};
use crate::mcp_host::ToolRoute;

pub const GROUP_ID: &str = "global";

const REGISTRY: Registry = &[
    ("search", search::build as BuildFn),
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
