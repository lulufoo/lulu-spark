//! Read-only spark APIs aligned with `server.py` GET handlers.

mod annotations;
mod config;
mod doc;
mod doc_open;

pub use crate::config::roots::spark_root_path;
pub use annotations::*;
pub use config::*;
pub use doc::*;
pub use doc_open::resolve_doc_for_open;

#[cfg(test)]
#[path = "../../unit-tests/services/spark_read.rs"]
mod tests;
