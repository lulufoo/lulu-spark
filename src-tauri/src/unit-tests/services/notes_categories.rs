use super::*;
use crate::services::spark_data::{self, list_note_categories};
use crate::test_support::{read_note_entries, seed_note_entries, TestSandbox};
use serde_json::json;
use std::fs;

fn setup() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = sandbox.data_dir().join("notes");
    fs::create_dir_all(&notes).expect("notes");
    sandbox
}

fn table_rows() -> Vec<spark_data::NoteCategoryRow> {
    let conn = spark_data::open().expect("db");
    list_note_categories(&conn).expect("rows")
}

#[test]
fn list_uses_note_category_table_not_json() {
    let sandbox = setup();
    let listed = list_notes_categories().expect("list");
    let ids: Vec<&str> = listed["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["inbox"]);
    let rows = table_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, INBOX_ID);
    assert_eq!(rows[0].folder, INBOX_ID);
    assert!(!sandbox.data_dir().join("notes/categories.json").is_file());
}

#[test]
fn ensure_writes_inbox_only_and_does_not_scan_notes() {
    let sandbox = setup();
    seed_note_entries(&json!({
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa": { "common_path": "inbox/theme/a.md" }
    }));

    let listed = list_notes_categories().expect("list");
    let ids: Vec<&str> = listed["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["inbox"]);
    assert_eq!(table_rows().len(), 1);
    assert!(!sandbox.data_dir().join("notes/categories.json").is_file());
}

#[test]
fn add_and_delete_do_not_touch_notes_or_raw() {
    let sandbox = setup();
    let notes = sandbox.data_dir().join("notes");
    let id = "cccccccccccccccccccccccccccccccc";
    fs::create_dir_all(notes.join("raw/ai-software-dev/theme")).expect("dir");
    fs::write(notes.join("raw/ai-software-dev/theme/n.md"), "# n").expect("raw");
    seed_note_entries(&json!({ id: { "common_path": "inbox/theme/n.md" } }));

    create_notes_category("research", "Research", "desc").expect("add");
    create_notes_category("ai-software-dev", "AI", "").expect("add existing slug");
    delete_notes_category("ai-software-dev").expect("delete");

    assert!(read_note_entries()["entries"][id]["common_path"] == "inbox/theme/n.md");
    assert!(notes.join("raw/ai-software-dev/theme/n.md").is_file());
    assert!(!known_ids().unwrap().contains("ai-software-dev"));
    assert!(known_ids().unwrap().contains("research"));
    let folders: Vec<String> = table_rows().into_iter().map(|r| r.folder).collect();
    assert!(folders.contains(&"research".into()));
    assert!(!folders.contains(&"ai-software-dev".into()));
}

#[test]
fn delete_inbox_and_invalid_id_rejected() {
    let _sandbox = setup();
    list_notes_categories().expect("ensure inbox");
    let err = delete_notes_category(INBOX_ID).expect_err("inbox");
    assert!(matches!(err, NotesCatError::BadRequest(_)));
    assert_eq!(table_rows().iter().filter(|r| r.id == INBOX_ID).count(), 1);
    let err = create_notes_category("bad/id", "Bad", "").expect_err("slash");
    assert!(matches!(err, NotesCatError::BadRequest(_)));
}

#[test]
fn update_title_does_not_touch_notes_or_id() {
    let sandbox = setup();
    let notes = sandbox.data_dir().join("notes");
    let id = "dddddddddddddddddddddddddddddddd";
    fs::create_dir_all(notes.join("raw/ops/theme")).expect("dir");
    fs::write(notes.join("raw/ops/theme/n.md"), "# n").expect("raw");
    seed_note_entries(&json!({ id: { "common_path": "ops/theme/n.md" } }));
    update_notes_category("ops", "Operations", "runbooks").expect("update");
    let listed = list_notes_categories().expect("list");
    let ops = listed["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "ops")
        .expect("ops");
    assert_eq!(ops["title"], "Operations");
    assert_eq!(ops["description"], "runbooks");
    assert_eq!(ops["folder"], "ops");
    let row = table_rows().into_iter().find(|r| r.id == "ops").expect("row");
    assert_eq!(row.title, "Operations");
    assert_eq!(row.description, "runbooks");
    assert_eq!(read_note_entries()["entries"][id]["common_path"], "ops/theme/n.md");
}

#[test]
fn duplicate_id_conflicts() {
    let _sandbox = setup();
    create_notes_category("ops", "Ops", "").expect("first");
    let err = create_notes_category("ops", "Ops 2", "").expect_err("dup");
    assert!(matches!(err, NotesCatError::Conflict(_)));
    assert_eq!(table_rows().iter().filter(|r| r.id == "ops").count(), 1);
}

#[test]
fn empty_id_mints_timestamp_alnum_and_folder() {
    let _sandbox = setup();
    let listed = create_notes_category("", " Research ", "  desc  ").expect("add");
    let cat = &listed["category"];
    let id = cat["id"].as_str().expect("id");
    assert_eq!(id.len(), 23);
    assert!(id.chars().take(17).all(|c| c.is_ascii_digit()), "ts {id}");
    assert!(id.chars().skip(17).all(|c| c.is_ascii_alphanumeric()), "rand {id}");
    assert_eq!(cat["title"], "Research");
    assert_eq!(cat["description"], "desc");
    assert_eq!(cat["folder"], id);
    assert!(known_ids().unwrap().contains(id));
    let row = table_rows().into_iter().find(|r| r.id == id).expect("row");
    assert_eq!(row.folder, id);
}
