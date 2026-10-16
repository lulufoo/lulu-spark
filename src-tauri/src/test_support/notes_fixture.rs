//! Seed and read the `note` table in the shape the old `index.json` had.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::spark_data::{
    self, get_note_category_by_folder, insert_note, insert_note_category, list_note_entries,
    NoteCategoryRow, NoteRow,
};

fn text(entry: &Value, key: &str) -> Option<String> {
    entry.get(key).and_then(Value::as_str).map(String::from)
}

/// Seed `common_path` is stored rel. Optional `folder` (default: first path
/// segment, or `inbox` when the path has no slash).
fn seed_folder_and_rel(entry: &Value, raw_path: &str) -> (String, String) {
    if let Some(folder) = text(entry, "folder") {
        return (folder, raw_path.to_string());
    }
    match raw_path.split_once('/') {
        Some((folder, rel)) if !folder.is_empty() && !rel.is_empty() => {
            (folder.to_string(), rel.to_string())
        }
        _ => ("inbox".into(), raw_path.to_string()),
    }
}

fn ensure_folder_category(conn: &Connection, folder: &str) -> String {
    if let Some(row) = get_note_category_by_folder(conn, folder).expect("lookup") {
        return row.id;
    }
    insert_note_category(conn, &NoteCategoryRow::new(folder, folder, folder, "")).expect("cat");
    folder.to_string()
}

/// `entries` is `{ "<id>": { common_path, folder?, created_at, layers, ... } }`.
/// Stored `raw_path` never includes the category folder.
pub fn seed_note_entries(entries: &Value) {
    let conn = spark_data::open().expect("open spark-data");
    for (id, entry) in entries.as_object().expect("entries object") {
        let raw_in = text(entry, "common_path").expect("common_path");
        let (folder, raw_path) = seed_folder_and_rel(entry, &raw_in);
        let mut note = NoteRow::new(id, &raw_path);
        note.category_id = ensure_folder_category(&conn, &folder);
        let has_digest = entry
            .get("layers")
            .and_then(Value::as_array)
            .is_some_and(|layers| layers.iter().any(|l| l.as_str() == Some("digest")));
        note.digest_path = has_digest.then(|| raw_path.clone());
        note.source_type = text(entry, "source_type");
        note.created_at = text(entry, "created_at");
        note.title = text(entry, "title");
        let mut langs: Vec<String> = entry
            .get("translations")
            .and_then(Value::as_object)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default();
        langs.sort();
        note.translations = json!(langs);
        insert_note(&conn, &note).expect("insert note");
    }
}

/// What `index.json` used to hold: `{ "entries": { id: entry } }`.
pub fn read_note_entries() -> Value {
    let conn = spark_data::open().expect("open spark-data");
    json!({ "entries": list_note_entries(&conn).expect("list notes") })
}

/// Make every `INSERT` / `UPDATE` / `DELETE` (`event`) on `note` fail, to prove what
/// happens when the table step fails after the file step.
pub fn block_note_writes(event: &str) {
    assert!(matches!(event, "INSERT" | "UPDATE" | "DELETE"));
    let conn = spark_data::open().expect("open spark-data");
    conn.execute_batch(&format!(
        "CREATE TRIGGER block_note_{event} BEFORE {event} ON note \
         BEGIN SELECT RAISE(ABORT, 'blocked by test'); END;"
    ))
    .expect("create trigger");
}
