//! Knowledge business group — API registry.

mod get_knowledge_content;
mod list_knowledge_categories;

use crate::mcp_host::ToolRoute;

pub const GROUP_ID: &str = "knowledge";

const SNAPSHOT_CHANNEL: &str = "cursor_ide";

type BuildFn = fn(&str) -> Option<ToolRoute>;

const REGISTRY: &[(&str, BuildFn)] = &[
    (
        "list_knowledge_categories",
        list_knowledge_categories::build as BuildFn,
    ),
    ("get_knowledge_content", get_knowledge_content::build as BuildFn),
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
