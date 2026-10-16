//! Notes entry delete / move-project: files first, then the `note` row.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::config::paths;
use crate::services::archive_parse::expected_lang_common_path;
use crate::services::search_index;
use crate::services::spark_data::{
    self, delete_note, delete_unused_tags, get_note, translation_common_paths, update_note,
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
    let common_path = note.raw_path.as_str();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path", "_status": 400 });
    }
    let mut deleted = Vec::new();
    for layer in LAYERS {
        let target = notes.join(layer).join(common_path);
        if !target.starts_with(&notes) {
            continue;
        }
        if target.is_file() {
            let _ = fs::remove_file(&target);
            deleted.push(format!("{layer}/{common_path}"));
        }
    }
    // The files are gone; only now does the record go.
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
    let old_cp = note.raw_path.clone();
    let old_cp = old_cp.as_str();
    if old_cp.is_empty() || old_cp.contains("..") {
        return json!({ "error": "Invalid common_path", "_status": 400 });
    }
    let mut parts: Vec<&str> = old_cp.split('/').collect();
    if parts.is_empty() {
        return json!({ "error": "Invalid common_path", "_status": 400 });
    }
    parts[0] = new_project;
    let new_cp = parts.join("/");
    if old_cp == new_cp {
        return json!({ "error": "Already in this project", "_status": 400 });
    }

    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    for layer in LAYERS {
        let src = notes.join(layer).join(old_cp);
        let dst = notes.join(layer).join(&new_cp);
        if src.exists() {
            moves.push((src, dst));
        }
    }
    let old_translations = match translation_common_paths(&note) {
        Ok(paths) => paths,
        Err(e) => return json!({ "error": e, "_status": 500 }),
    };
    let mut renamed: Vec<(String, String)> = Vec::new();
    for (lang, old_path) in &old_translations {
        let Some(new_path) = expected_lang_common_path(&new_cp, lang) else {
            return json!({ "error": "Invalid common_path", "_status": 400 });
        };
        let (src, dst) = (notes.join("raw").join(old_path), notes.join("raw").join(&new_path));
        if src.exists() {
            moves.push((src, dst));
        }
        renamed.push((old_path.clone(), new_path));
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

    for (src, dst) in &completed {
        if dst.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        if let Ok(mut content) = fs::read_to_string(dst) {
            for layer in LAYERS {
                let old_ref = format!("{layer}/{old_cp}");
                let new_ref = format!("{layer}/{new_cp}");
                content = content.replace(&old_ref, &new_ref);
                content = content.replace(&format!("`{old_ref}`"), &format!("`{new_ref}`"));
            }
            for (old_path, new_path) in &renamed {
                content = content.replace(&format!("raw/{old_path}"), &format!("raw/{new_path}"));
            }
            let _ = fs::write(dst, content);
        }
        let _ = src;
    }

    note.raw_path = new_cp.clone();
    if note.digest_path.is_some() {
        note.digest_path = Some(new_cp.clone());
    }
    if let Err(e) = update_note(&conn, &note) {
        return json!({ "error": e, "_status": 500 });
    }
    sync_index_for(entry_id);
    json!({ "ok": true, "new_common_path": new_cp })
}

#[cfg(test)]
#[path = "../unit-tests/services/entry_admin.rs"]
mod tests;
