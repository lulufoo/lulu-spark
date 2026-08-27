//! Read-only workbench APIs aligned with `server.py` GET handlers.

mod annotations;
mod config;
mod corpus;
mod status;

pub use crate::config::meili_env::{
    github_user_url_string, knowledge_corpus_root_string, workbench_knowledge_root_path,
};
pub use annotations::*;
pub use config::*;
pub use corpus::*;
pub use status::*;

#[cfg(test)]
#[path = "../../unit-tests/services/workbench_read.rs"]
mod tests;
