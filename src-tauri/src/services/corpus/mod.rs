//! Knowledge corpus read / write / cloned-repo git / iTerm.
//!
//! Command names and Sidecar `/api/kb/*` paths stay on L1.

mod read;
mod write;
mod repo_git;
mod iterm;

pub use read::*;
pub use write::*;
pub use repo_git::*;
pub use iterm::*;
