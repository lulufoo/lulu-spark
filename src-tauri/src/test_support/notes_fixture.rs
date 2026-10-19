//! Seed and read the default workspace's documents in the shape the old `index.json` had.

use rusqlite::Connection;
use serde_json::{json, Value};

use crate::services::spark_data::{
    self, get_default_workspace, get_project, insert_doc, insert_project, list_doc_entries, DocRow,
    ProjectRow,
};

fn text(entry: &Value, key: &str) -> Option<String> {
    entry.get(key).and_then(Value::as_str).map(String::from)
}

/// Optional `folder` names the seed project (default: first path segment, or `inbox`).
/// The rest of `common_path` is stored as `raw_path`.
fn seed_project_and_rel(entry: &Value, raw_path: &str) -> (String, String) {
    if let Some(folder) = text(entry, "folder") {
        return (project_id_for_seed(&folder), raw_path.to_string());
    }
    match raw_path.split_once('/') {
        Some((folder, rel)) if !folder.is_empty() && !rel.is_empty() => {
            (project_id_for_seed(folder), rel.to_string())
        }
        _ => ("inbox".into(), raw_path.to_string()),
    }
}

fn project_id_for_seed(folder: &str) -> String {
    if folder == "inbox" {
        "inbox".into()
    } else {
        format!("seed-{folder}")
    }
}

fn ensure_project(conn: &Connection, project_id: &str) -> String {
    if get_project(conn, project_id).expect("lookup").is_some() {
        return project_id.to_string();
    }
    let workspace = get_default_workspace(conn).expect("default workspace");
    insert_project(
        conn,
        &ProjectRow::new(project_id, &workspace.id, project_id, ""),
    )
    .expect("project");
    project_id.to_string()
}

/// `entries` is `{ "<id>": { common_path, folder?, created_at, layers, ... } }`.
/// Stored `raw_path` never includes the project identity.
pub fn seed_note_entries(entries: &Value) {
    let conn = spark_data::open().expect("open spark-data");
    for (id, entry) in entries.as_object().expect("entries object") {
        let raw_in = text(entry, "common_path").expect("common_path");
        let (project_id, raw_path) = seed_project_and_rel(entry, &raw_in);
        let project_id = ensure_project(&conn, &project_id);
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
        insert_doc(&conn, &get_default_workspace(&conn).expect("default workspace").doc_table, &note)
            .expect("insert doc");
    }
}

/// What `index.json` used to hold: `{ "entries": { id: entry } }`.
pub fn read_note_entries() -> Value {
    let conn = spark_data::open().expect("open spark-data");
    let workspace = get_default_workspace(&conn).expect("default workspace");
    json!({ "entries": list_doc_entries(&conn, &workspace).expect("list docs") })
}

/// Make every `INSERT` / `UPDATE` / `DELETE` (`event`) on the default document table fail, to prove what
/// happens when the table step fails after the file step.
pub fn block_note_writes(event: &str) {
    assert!(matches!(event, "INSERT" | "UPDATE" | "DELETE"));
    let conn = spark_data::open().expect("open spark-data");
    let table = get_default_workspace(&conn).expect("default workspace").doc_table;
    conn.execute_batch(&format!(
        "CREATE TRIGGER block_note_{event} BEFORE {event} ON {table} \
         BEGIN SELECT RAISE(ABORT, 'blocked by test'); END;"
    ))
    .expect("create trigger");
}
