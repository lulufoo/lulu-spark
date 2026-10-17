//! Seed and read the default workspace's documents in the shape the old `index.json` had.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::spark_data::{
    self, get_project_by_folder, get_workspace, insert_doc, insert_project, list_doc_entries,
    DocRow, ProjectRow, DEFAULT_WORKSPACE_ID,
};

const DEFAULT_DOC_TABLE: &str = "doc_default";

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

fn ensure_folder_project(conn: &Connection, folder: &str) -> String {
    if let Some(row) = get_project_by_folder(conn, DEFAULT_WORKSPACE_ID, folder).expect("lookup") {
        return row.id;
    }
    let id = format!("seed-{folder}");
    insert_project(conn, &ProjectRow::new(&id, DEFAULT_WORKSPACE_ID, folder, folder, "")).expect("project");
    id
}

/// `entries` is `{ "<id>": { common_path, folder?, created_at, layers, ... } }`.
/// Stored `raw_path` never includes the category folder.
pub fn seed_note_entries(entries: &Value) {
    let conn = spark_data::open().expect("open spark-data");
    for (id, entry) in entries.as_object().expect("entries object") {
        let raw_in = text(entry, "common_path").expect("common_path");
        let (folder, raw_path) = seed_folder_and_rel(entry, &raw_in);
        let project_id = ensure_folder_project(&conn, &folder);
        let mut note = DocRow::new(id, &raw_path, &project_id);
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
        insert_doc(&conn, DEFAULT_DOC_TABLE, &note).expect("insert doc");
    }
}

/// What `index.json` used to hold: `{ "entries": { id: entry } }`.
pub fn read_note_entries() -> Value {
    let conn = spark_data::open().expect("open spark-data");
    let workspace = get_workspace(&conn, DEFAULT_WORKSPACE_ID).expect("workspace").expect("default workspace");
    json!({ "entries": list_doc_entries(&conn, &workspace).expect("list docs") })
}

/// Make every `INSERT` / `UPDATE` / `DELETE` (`event`) on the default document table fail, to prove what
/// happens when the table step fails after the file step.
pub fn block_note_writes(event: &str) {
    assert!(matches!(event, "INSERT" | "UPDATE" | "DELETE"));
    let conn = spark_data::open().expect("open spark-data");
    conn.execute_batch(&format!(
        "CREATE TRIGGER block_note_{event} BEFORE {event} ON doc_default \
         BEGIN SELECT RAISE(ABORT, 'blocked by test'); END;"
    ))
    .expect("create trigger");
}
