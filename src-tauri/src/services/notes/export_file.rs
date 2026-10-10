//! `get_note_file`: copy a note's raw file into a caller-chosen directory.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::file_export::copy_into_dir_versioned;
use crate::services::spark_read::get_note_path_by_id;

/// Copy the raw layer of note `id` into `dest_dir`. Success is `{id, ok, path}` only.
pub fn get_note_file(repo_root: &Path, id: &str, dest_dir: &str) -> Value {
    let located = get_note_path_by_id(repo_root, id);
    if located.get("ok") != Some(&json!(true)) {
        return located;
    }
    let (Some(note_id), Some(source)) = (
        located.get("id").and_then(|v| v.as_str()),
        located.get("path").and_then(|v| v.as_str()),
    ) else {
        return json!({ "id": id.trim(), "ok": false, "error": "Missing path" });
    };
    match copy_into_dir_versioned(&PathBuf::from(source), dest_dir, note_id, "md") {
        Ok(path) => json!({ "id": note_id, "ok": true, "path": path.to_string_lossy() }),
        Err(mut err) => {
            err["id"] = json!(note_id);
            err["ok"] = json!(false);
            err
        }
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/notes_export_file.rs"]
mod tests;
