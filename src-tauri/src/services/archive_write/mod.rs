//! Corpus archive writes for MCP (`archive_document`, `archive_digest`).

mod digest;
mod document;
mod note;
mod store;

pub use digest::*;
pub use document::*;
pub use note::*;

#[cfg(test)]
#[path = "../../unit-tests/services/archive_write.rs"]
mod tests;
