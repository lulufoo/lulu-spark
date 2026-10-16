//! Read-only spark APIs aligned with `server.py` GET handlers.

mod annotations;
mod config;
mod note_open;
mod notes;

pub use crate::config::roots::spark_root_path;
pub use annotations::*;
pub use config::*;
pub use note_open::resolve_note_for_open;
pub use notes::*;

#[cfg(test)]
#[path = "../../unit-tests/services/spark_read.rs"]
mod tests;
