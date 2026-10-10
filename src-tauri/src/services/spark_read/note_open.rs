//! Resolve a note id for the in-app open flow (`[title](note:<id>)` links in chat).

use std::path::Path;

use serde_json::{json, Value};

use super::notes::load_notes_index_entries;
use super::notes_catalog::is_valid_entry_id;

/// Look up note `id` in the notes index. Success is the index entry plus `id` and `ok`.
/// Never returns a file body or an absolute path.
pub fn resolve_note_for_open(repo_root: &Path, id: &str) -> Value {
    let id = id.trim();
    if !is_valid_entry_id(id) {
        return json!({ "id": id, "ok": false, "error": "Invalid id" });
    }
    let entries = match load_notes_index_entries(repo_root) {
        Ok(entries) => entries,
        Err(err) => return err,
    };
    let Some(Value::Object(entry)) = entries.get(id) else {
        return json!({ "id": id, "ok": false, "error": "Entry not found" });
    };
    let mut resolved = entry.clone();
    resolved.insert("id".into(), json!(id));
    resolved.insert("ok".into(), json!(true));
    Value::Object(resolved)
}

#[cfg(test)]
#[path = "../../unit-tests/services/note_open.rs"]
mod tests;
