//! A `note` row in the shape of the notes-index entry.

use rusqlite::Connection;
use serde_json::{json, Map, Value};

use crate::services::archive_parse::expected_lang_common_path;

use super::note_category_row::category_folder;
use super::note_row::{list_notes, NoteRow};

/// `translations` is stored as a language list; each path is derived from `raw_path`.
pub fn translation_common_paths(note: &NoteRow) -> Result<Vec<(String, String)>, String> {
    let langs = note.translations.as_array().cloned().unwrap_or_default();
    langs
        .iter()
        .filter_map(Value::as_str)
        .map(|lang| {
            expected_lang_common_path(&note.raw_path, lang)
                .map(|path| (lang.to_string(), path))
                .ok_or_else(|| format!("note {}: no translation path for `{}`", note.doc_id, note.raw_path))
        })
        .collect()
}

/// `{id, category_id, folder, created_at, layers, source_type, translations?, title?}`.
pub fn entry_from_note(note: &NoteRow, folder: &str) -> Result<Value, String> {
    let mut entry = Map::new();
    entry.insert("id".into(), json!(note.doc_id));
    entry.insert("category_id".into(), json!(note.category_id));
    entry.insert("folder".into(), json!(folder));
    entry.insert(
        "layers".into(),
        if note.digest_path.is_some() { json!(["raw", "digest"]) } else { json!(["raw"]) },
    );
    for (key, value) in [
        ("created_at", &note.created_at),
        ("source_type", &note.source_type),
        ("title", &note.title),
    ] {
        if let Some(v) = value {
            entry.insert(key.into(), json!(v));
        }
    }
    let translations = translation_common_paths(note)?;
    if !translations.is_empty() {
        let map: Map<String, Value> = translations.into_iter().map(|(lang, path)| (lang, json!(path))).collect();
        entry.insert("translations".into(), Value::Object(map));
    }
    Ok(Value::Object(entry))
}

/// `doc_id → entry` for every note.
pub fn list_note_entries(conn: &Connection) -> Result<Map<String, Value>, String> {
    let mut entries = Map::new();
    for note in list_notes(conn)? {
        let folder = category_folder(conn, &note.category_id)?;
        entries.insert(note.doc_id.clone(), entry_from_note(&note, &folder)?);
    }
    Ok(entries)
}
