//! Read-only spark APIs aligned with `server.py` GET handlers.

mod annotations;
mod config;
mod note_open;
mod notes;
mod notes_catalog;

pub use crate::config::roots::{
    knowledge_root_string, spark_root_path,
};
pub use annotations::*;
pub use config::*;
pub use note_open::resolve_note_for_open;
pub use notes::*;
pub use notes_catalog::*;

#[cfg(test)]
#[path = "../../unit-tests/services/spark_read.rs"]
mod tests;
