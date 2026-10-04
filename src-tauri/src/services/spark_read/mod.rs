//! Read-only spark APIs aligned with `server.py` GET handlers.

mod annotations;
mod config;
mod notes;
mod notes_catalog;
mod status;

pub use crate::config::roots::{
    github_user_url_string, knowledge_root_string, spark_root_path,
};
pub use annotations::*;
pub use config::*;
pub use notes::*;
pub use notes_catalog::*;
pub use status::*;

#[cfg(test)]
#[path = "../../unit-tests/services/spark_read.rs"]
mod tests;
