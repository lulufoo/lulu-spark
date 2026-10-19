//! Document ids: 32 lowercase hex, global, with no workspace in them. The workspace of a
//! document is its project's workspace.

use crate::services::id::random_entry_id;

/// A fresh document id.
pub fn new_doc_id() -> String {
    random_entry_id()
}

pub fn is_valid_doc_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}
