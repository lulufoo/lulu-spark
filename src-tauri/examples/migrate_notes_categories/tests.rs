use std::fs;
use std::path::Path;

use lulu_spark_lib::services::spark_data::{
    get_note_category, get_note_category_by_folder, list_note_categories, open_at, DB_FILE,
};
use serde_json::{json, Value};
use tempfile::TempDir;

use crate::migration::run_migration;

fn write_json(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, serde_json::to_string_pretty(value).expect("json")).expect("write");
}

fn categories_json(items: Value) -> Value {
    json!({ "version": 1, "categories": items })
}

fn fixture_without_ai() -> TempDir {
    let tmp = TempDir::new().expect("tmp");
    write_json(
        &tmp.path().join("notes/categories.json"),
        &categories_json(json!([
            { "id": "inbox", "folder": "inbox", "title": "Inbox", "description": "" },
            { "id": "ops", "folder": "ops", "title": "Ops", "description": "runbooks" }
        ])),
    );
    tmp
}

#[test]
fn imports_inserts_ai_reconciles_and_deletes_json() {
    let tmp = fixture_without_ai();
    let json_path = tmp.path().join("notes/categories.json");
    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.row_count, 3);
    assert!(outcome.inserted_ai);
    assert!(!json_path.exists());

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let rows = list_note_categories(&conn).expect("list");
    assert_eq!(rows.len(), 3);
    let inbox = get_note_category(&conn, "inbox").expect("get").expect("inbox");
    assert_eq!(
        (inbox.folder.as_str(), inbox.title.as_str(), inbox.description.as_str()),
        ("inbox", "Inbox", "")
    );
    let ops = get_note_category(&conn, "ops").expect("get").expect("ops");
    assert_eq!(
        (ops.folder.as_str(), ops.title.as_str(), ops.description.as_str()),
        ("ops", "Ops", "runbooks")
    );
    let ai = get_note_category_by_folder(&conn, "ai").expect("get").expect("ai");
    assert_eq!(ai.folder, "ai");
    assert_eq!(ai.id.len(), 23);
    assert!(ai.id.chars().take(17).all(|c| c.is_ascii_digit()), "ts {}", ai.id);
    assert!(
        ai.id.chars().skip(17).all(|c| c.is_ascii_alphanumeric()),
        "rand {}",
        ai.id
    );
}

#[test]
fn ai_already_in_json_is_not_duplicated() {
    let tmp = TempDir::new().expect("tmp");
    write_json(
        &tmp.path().join("notes/categories.json"),
        &categories_json(json!([
            { "id": "inbox", "folder": "inbox", "title": "Inbox", "description": "" },
            { "id": "old-ai", "folder": "ai", "title": "AI", "description": "notes" }
        ])),
    );
    let outcome = run_migration(tmp.path()).expect("migrate");
    assert_eq!(outcome.row_count, 2);
    assert!(!outcome.inserted_ai);

    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    let ais: Vec<_> = list_note_categories(&conn)
        .expect("list")
        .into_iter()
        .filter(|r| r.folder == "ai")
        .collect();
    assert_eq!(ais.len(), 1);
    assert_eq!(ais[0].id, "old-ai");
    assert_eq!(ais[0].title, "AI");
    assert_eq!(ais[0].description, "notes");
    assert!(!tmp.path().join("notes/categories.json").exists());
}

#[test]
fn missing_json_is_refused_and_writes_nothing() {
    let tmp = TempDir::new().expect("tmp");
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("categories.json"), "{err}");
    assert!(!tmp.path().join(DB_FILE).exists());
}

#[test]
fn a_field_mismatch_deletes_nothing() {
    let tmp = fixture_without_ai();
    let conn = open_at(&tmp.path().join(DB_FILE)).expect("db");
    lulu_spark_lib::services::spark_data::insert_note_category(
        &conn,
        &lulu_spark_lib::services::spark_data::NoteCategoryRow::new("ops", "ops", "Other", ""),
    )
    .expect("seed clash");
    drop(conn);

    let json_path = tmp.path().join("notes/categories.json");
    let err = run_migration(tmp.path()).err().expect("must stop");
    assert!(err.contains("ops"), "{err}");
    assert!(json_path.is_file());
}
