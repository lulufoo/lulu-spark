//! Knowledge clone read / write / cloned-repo git / iTerm / MCP read surface.
//!
//! Command names and Sidecar `/api/kb/*` paths stay on L1.

mod read;
mod write;
mod repo_git;
mod iterm;
mod doc_map;
mod mcp;

pub use read::*;
pub use write::*;
pub use repo_git::*;
pub use iterm::*;
pub use mcp::*;
