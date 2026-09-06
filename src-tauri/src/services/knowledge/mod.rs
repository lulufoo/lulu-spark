//! Knowledge clone read / write / cloned-repo git / iTerm / MCP read surface.
//!
//! Command names and Sidecar `/api/kb/*` paths stay on L1.

mod read;
mod write;
mod repo_git;
mod iterm;
mod doc_map;
mod hide_patterns;
mod mcp;
mod search_document;

pub use read::*;
pub use write::*;
pub use repo_git::*;
pub use iterm::*;
pub use mcp::*;
pub use search_document::*;
pub use hide_patterns::{
    add as add_kb_hide_pattern, compiled_hide_regexes, list_json as kb_hide_patterns_json,
    name_is_hidden, remove as remove_kb_hide_pattern, update as update_kb_hide_pattern,
};
