//! Note writes for MCP (`create_note`, `create_note_content`). Digest is a
//! `create_note` field (`digest` + optional `digest_body`), not a second tool.

mod categories;
mod create_meta;
mod digest;
mod document;
mod jot;
mod layout;
mod store;

pub use categories::{
    create_notes_category, create_notes_category_value, delete_notes_category,
    delete_notes_category_value, ensure_categories, known_ids, list_notes_categories,
    list_notes_categories_value, update_notes_category, update_notes_category_value,
    NotesCatError, INBOX_ID,
};
pub use digest::*;
pub use document::*;
pub use jot::*;
pub use layout::ensure_notes_layout;

#[cfg(test)]
#[path = "../../unit-tests/services/notes.rs"]
mod tests;
