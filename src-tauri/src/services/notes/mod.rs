//! Note writes for MCP (`create_note`, `create_note_content`). Digest is a
//! `create_note` field (`digest` + optional `digest_body`), not a second tool.

mod create_meta;
mod digest;
mod document;
mod jot;
mod layout;
mod store;

pub use digest::*;
pub use document::*;
pub use jot::*;
pub use layout::ensure_notes_layout;

#[cfg(test)]
#[path = "../../unit-tests/services/notes.rs"]
mod tests;
