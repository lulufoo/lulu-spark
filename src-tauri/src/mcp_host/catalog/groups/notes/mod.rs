//! Notes business group — API registry.

mod create_note;
mod create_notes_category;
mod delete_note;
mod update_note;
mod delete_notes_category;
mod get_all_notes_catalog;
mod get_latest_digest_per_catalog;
mod get_note_content;
mod get_note_digest_by_id;
mod get_notes_by_catalog;
mod list_notes_categories;
mod schema;
mod update_notes_category;

use crate::mcp_host::ToolRoute;

pub(crate) use create_note::{
    invoke_from_content as create_note_from_content,
    invoke_from_source as create_note_from_source,
    produce_notes_if_ok as produce_create_note_if_ok,
};
pub(crate) use get_note_content::invoke as note_path_invoke;
pub(crate) use update_note::{
    invoke_from_content as update_note_from_content,
    invoke_from_source as update_note_from_source,
    produce_notes_if_ok as produce_update_note_if_ok,
};

pub const GROUP_ID: &str = "notes";

/// Channel used for Settings snapshot metadata (canonical paths/descriptions).
const SNAPSHOT_CHANNEL: &str = "cursor_ide";

type BuildFn = fn(&str) -> Option<ToolRoute>;

const REGISTRY: &[(&str, BuildFn)] = &[
    ("get_all_notes_catalog", get_all_notes_catalog::build as BuildFn),
    (
        "get_latest_digest_per_catalog",
        get_latest_digest_per_catalog::build as BuildFn,
    ),
    ("get_notes_by_catalog", get_notes_by_catalog::build as BuildFn),
    ("get_note_digest_by_id", get_note_digest_by_id::build as BuildFn),
    ("get_note_content", get_note_content::build as BuildFn),
    ("create_note", create_note::build as BuildFn),
    ("update_note", update_note::build as BuildFn),
    ("delete_note", delete_note::build as BuildFn),
    ("list_notes_categories", list_notes_categories::build as BuildFn),
    ("create_notes_category", create_notes_category::build as BuildFn),
    ("update_notes_category", update_notes_category::build as BuildFn),
    ("delete_notes_category", delete_notes_category::build as BuildFn),
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

/// Settings / slot table metadata without channel overlays (except workbench-only tools).
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

/// Enabled tool routes for one MCP channel.
pub fn routes_for_channel(channel: &str) -> Vec<ToolRoute> {
    REGISTRY
        .iter()
        .filter_map(|(_, build_fn)| build_fn(channel))
        .collect()
}
