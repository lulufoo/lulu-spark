//! Notes entry delete / move-project: files first, then the `note` row.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::config::paths;
use crate::services::archive_parse::expected_lang_common_path;
use crate::services::notes::notes_file_path;
use crate::services::search_index;
use crate::services::spark_data::{
    self, category_folder, delete_note, delete_unused_tags, get_note, get_note_category_by_folder,
    translation_common_paths, update_note,
};
const LAYERS: &[&str] = &["raw", "digest"];

fn notes_root() -> Result<PathBuf, Value> {
    paths::notes_root().map_err(|e| json!({ "error": format!("{e:?}") }))
}

fn open_data() -> Result<rusqlite::Connection, Value> {
    spark_data::open().map_err(|e| json!({ "error": e, "_status": 500 }))
}

fn load_note(conn: &rusqlite::Connection, id: &str) -> Result<spark_data::NoteRow, Value> {
    match get_note(conn, id) {
        Ok(Some(note)) => Ok(note),
        Ok(None) => Err(json!({ "error": "Entry not found", "_status": 404 })),
        Err(e) => Err(json!({ "error": e, "_status": 500 })),
    }
}

fn valid_projects() -> HashSet<String> {
    crate::services::notes::known_ids().unwrap_or_else(|_| {
        let mut set = HashSet::new();
        set.insert(crate::services::notes::INBOX_ID.to_string());
        set
    })
}

fn is_valid_entry_id(id: &str) -> bool {
    id.len() == 32 && id.chars().all(|c| c.is_ascii_hexdigit())
}

fn layer_file(notes: &std::path::Path, layer: &str, folder: &str, rel: &str) -> Result<PathBuf, Value> {
    notes_file_path(notes, layer, folder, rel)
}

pub fn delete_entry(payload: &Value) -> Value {
    let entry_id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !is_valid_entry_id(entry_id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }
    let notes = match notes_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    let conn = match open_data() {
        Ok(conn) => conn,
        Err(v) => return v,
    };
    let note = match load_note(&conn, entry_id) {
        Ok(note) => note,
        Err(v) => return v,
    };
    let rel = note.raw_path.as_str();
    if rel.is_empty() || rel.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let folder = match category_folder(&conn, &note.category_id) {
        Ok(f) => f,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let mut deleted = Vec::new();
    for layer in LAYERS {
        let Ok(target) = layer_file(&notes, layer, &folder, rel) else {
            continue;
        };
        if target.is_file() {
            let _ = fs::remove_file(&target);
            deleted.push(format!("{layer}/{folder}/{rel}"));
        }
    }
    if let Err(e) = delete_note(&conn, entry_id).and_then(|_| delete_unused_tags(&conn)) {
        return json!({ "error": e, "_status": 500 });
    }
    sync_index_for(entry_id);
    json!({
        "ok": true,
        "id": entry_id,
        "removed": deleted.len(),
    })
}

/// Re-chunk the note from its row, or drop its index rows when the row is gone.
fn sync_index_for(entry_id: &str) {
    if let Ok(repo_root) = paths::repo_root() {
        search_index::sync_note_docs_best_effort(&repo_root, &[entry_id]);
    }
}

pub fn move_entry_project(payload: &Value) -> Value {
    let entry_id = payload.get("id").and_then(|v| v.as_str()).unwrap_or("").trim();
    let new_project = payload
        .get("new_project")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if !is_valid_entry_id(entry_id) {
        return json!({ "error": "Invalid id", "_status": 400 });
    }
    if new_project.is_empty() || new_project.contains("..") || new_project.contains('/') {
        return json!({ "error": "Invalid new_project", "_status": 400 });
    }
    let valid = valid_projects();
    if !valid.is_empty() && !valid.contains(new_project) {
        return json!({ "error": format!("Unknown project: {new_project}"), "_status": 400 });
    }
    let notes = match notes_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    let conn = match open_data() {
        Ok(conn) => conn,
        Err(v) => return v,
    };
    let mut note = match load_note(&conn, entry_id) {
        Ok(note) => note,
        Err(v) => return v,
    };
    let rel = note.raw_path.clone();
    if rel.is_empty() || rel.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let old_folder = match category_folder(&conn, &note.category_id) {
        Ok(f) => f,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    if old_folder == new_project {
        return json!({ "error": "Already in this project", "_status": 400 });
    }
    let new_cat = match get_note_category_by_folder(&conn, new_project) {
        Ok(Some(cat)) => cat,
        Ok(None) => return json!({ "error": format!("Unknown project: {new_project}"), "_status": 400 }),
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };

    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    for layer in LAYERS {
        let src = match layer_file(&notes, layer, &old_folder, &rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        let dst = match layer_file(&notes, layer, new_project, &rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        if src.exists() {
            moves.push((src, dst));
        }
    }
    let old_translations = match translation_common_paths(&note) {
        Ok(paths) => paths,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    for (lang, old_rel) in &old_translations {
        let Some(new_rel) = expected_lang_common_path(&rel, lang) else {
            return json!({ "error": "Invalid path", "_status": 400 });
        };
        let src = match layer_file(&notes, "raw", &old_folder, old_rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        let dst = match layer_file(&notes, "raw", new_project, &new_rel) {
            Ok(p) => p,
            Err(v) => return v,
        };
        if src.exists() {
            moves.push((src, dst));
        }
    }

    let mut completed = Vec::new();
    for (src, dst) in &moves {
        if let Some(parent) = dst.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                for (s, d) in completed.iter().rev() {
                    let _ = fs::rename(d, s);
                }
                return json!({ "error": format!("Move failed, rolled back: {e}"), "_status": 500 });
            }
        }
        if let Err(e) = fs::rename(src, dst) {
            for (s, d) in completed.iter().rev() {
                let _ = fs::rename(d, s);
            }
            return json!({ "error": format!("Move failed, rolled back: {e}"), "_status": 500 });
        }
        completed.push((src.clone(), dst.clone()));
    }

    note.category_id = new_cat.id;
    if let Err(e) = update_note(&conn, &note) {
        return json!({ "error": e, "_status": 500 });
    }
    sync_index_for(entry_id);
    json!({ "ok": true, "id": entry_id })
}

#[cfg(test)]
#[path = "../unit-tests/services/entry_admin.rs"]
mod tests;
