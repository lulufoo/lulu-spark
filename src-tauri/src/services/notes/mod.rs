//! Note writes for MCP (`create_note`, `create_note_content`, `update_note`).
//! Digest is a field (`digest` + optional `digest_body`), not a second tool.

mod assets;
mod categories;
mod create_meta;
mod digest;
mod document;
mod export_file;
mod jot;
mod layout;
mod store;
mod update;

pub use categories::{
    create_notes_category, create_notes_category_value, delete_notes_category,
    delete_notes_category_value, ensure_categories, known_ids, list_notes_categories,
    list_notes_categories_value, update_notes_category, update_notes_category_value,
    NotesCatError, INBOX_ID,
};
pub use document::*;
pub use export_file::get_note_file;
pub use jot::*;
pub use update::*;
pub use layout::ensure_notes_layout;

#[cfg(test)]
#[path = "../../unit-tests/services/notes.rs"]
mod tests;

#[cfg(test)]
#[path = "../../unit-tests/services/notes_assets.rs"]
mod assets_tests;
