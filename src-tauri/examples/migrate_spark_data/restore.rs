//! Rebuild the old JSON shapes from what is in the database.

use std::collections::BTreeMap;

use lulu_spark_lib::services::spark_data::{get_note, NoteRow};
use rusqlite::Connection;
use serde_json::{json, Map, Value};

use crate::shapes::{translation_path, ENTRY_KEYS, KnowledgeAnnotation, NoteAnnotation};

/// The old `index.json` entry fields the new model keeps (nulls count as absent).
pub fn expected_entry(old: &Value) -> Value {
    let mut out = Map::new();
    for key in ENTRY_KEYS {
        if let Some(v) = old.get(key).filter(|v| !v.is_null()) {
            out.insert(key.to_string(), v.clone());
        }
    }
    Value::Object(out)
}

pub fn restore_entry(note: &NoteRow) -> Result<Value, String> {
    let mut out = Map::new();
    out.insert("common_path".into(), json!(note.raw_path));
    out.insert("layers".into(), if note.digest_path.is_some() { json!(["raw", "digest"]) } else { json!(["raw"]) });
    for (key, value) in [("created_at", &note.created_at), ("source_type", &note.source_type), ("title", &note.title)] {
        if let Some(v) = value {
            out.insert(key.into(), json!(v));
        }
    }
    let langs = note.translations.as_array().cloned().unwrap_or_default();
    if !langs.is_empty() {
        let mut map = Map::new();
        for lang in langs.iter().filter_map(Value::as_str) {
            map.insert(lang.to_string(), json!(translation_path(&note.raw_path, lang)?));
        }
        out.insert("translations".into(), Value::Object(map));
    }
    Ok(Value::Object(out))
}

pub fn restore_note_annotation(note: &NoteRow) -> NoteAnnotation {
    NoteAnnotation {
        tag_keys: note.tags.clone(),
        importance: note.importance.clone(),
        done: note.done,
        links: note.links.clone(),
        raw_comments: note.comments.clone(),
        raw_highlights: note.highlights.clone(),
    }
}

pub fn read_notes(conn: &Connection) -> Result<BTreeMap<String, NoteRow>, String> {
    let mut stmt = conn.prepare("SELECT doc_id FROM note").map_err(|e| e.to_string())?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let mut notes = BTreeMap::new();
    for id in ids {
        if let Some(note) = get_note(conn, &id)? {
            notes.insert(id, note);
        }
    }
    Ok(notes)
}

/// `key → (value, refs)`, the references counted from the notes.
pub fn read_tags(conn: &Connection) -> Result<BTreeMap<String, (String, i64)>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT tag.key, tag.value,
                    (SELECT COUNT(*) FROM note, json_each(note.tags) WHERE json_each.value = tag.key)
             FROM tag",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, (row.get::<_, String>(1)?, row.get::<_, i64>(2)?))))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<BTreeMap<_, _>, _>>().map_err(|e| e.to_string())
}

/// `(repo, path) → annotation` of every knowledge record.
pub fn read_knowledge(conn: &Connection) -> Result<BTreeMap<(String, String), KnowledgeAnnotation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT k.folder, d.path, d.comments, d.links, d.highlights
             FROM knowledge_doc d
             INNER JOIN knowledge_directory k ON k.id = d.directory_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let parse = |i: usize| -> rusqlite::Result<Value> {
                let text: String = row.get(i)?;
                serde_json::from_str(&text).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
                })
            };
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                KnowledgeAnnotation { comments: parse(2)?, links: parse(3)?, highlights: parse(4)? },
            ))
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<BTreeMap<_, _>, _>>().map_err(|e| e.to_string())
}
