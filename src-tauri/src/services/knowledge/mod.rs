//! Knowledge directory read / write / MCP read surface.
//!
//! Command names and Sidecar `/api/kb/*` paths stay on L1.

mod asset;
mod read;
mod write;
mod rename;
mod entry;
mod doc_map;
mod hide_patterns;
mod viewer_state;
mod mcp;
mod search_document;

pub use asset::*;
pub use read::*;
pub use write::*;
pub use rename::*;
pub use entry::*;
pub use mcp::*;
pub use search_document::*;
pub use hide_patterns::{
    add as add_kb_hide_pattern, compiled_hide_regexes, list_json as kb_hide_patterns_json,
    name_is_hidden, remove as remove_kb_hide_pattern, update as update_kb_hide_pattern,
};
pub use viewer_state::{get_json as kb_viewer_state_json, set_json as set_kb_viewer_state};
