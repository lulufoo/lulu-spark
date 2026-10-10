//! Global business group — APIs that span more than one domain.

mod get_read_later;
mod search_document;

use crate::mcp_host::ToolRoute;

pub const GROUP_ID: &str = "global";

const SNAPSHOT_CHANNEL: &str = "cursor_ide";

type BuildFn = fn(&str) -> Option<ToolRoute>;

const REGISTRY: &[(&str, BuildFn)] = &[
    ("search_document", search_document::build as BuildFn),
    ("get_read_later", get_read_later::build as BuildFn),
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

pub fn catalog_snapshot_routes() -> Vec<ToolRoute> {
    REGISTRY
        .iter()
        .map(|(key, build_fn)| {
            build_fn(SNAPSHOT_CHANNEL)
                .or_else(|| build_fn("spark"))
                .unwrap_or_else(|| panic!("catalog snapshot missing for {key}"))
        })
        .collect()
}

pub fn routes_for_channel(channel: &str) -> Vec<ToolRoute> {
    REGISTRY
        .iter()
        .filter_map(|(_, build_fn)| build_fn(channel))
        .collect()
}
