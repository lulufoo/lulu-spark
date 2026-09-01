use super::*;
use crate::test_support::TestSandbox;
use serde_json::json;
use std::fs;

fn setup() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = sandbox.workbench_root().join("notes");
    fs::create_dir_all(&notes).expect("notes");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("index");
    sandbox
}

#[test]
fn notes_categories_path_under_notes_not_knowledge() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let path = crate::config::paths::notes_categories_path().expect("path");
    assert_eq!(path, wb.join("notes").join("categories.json"));
    let knowledge = crate::config::paths::sediment_kb_categories_path().expect("kb");
    assert_ne!(path, knowledge);
}

#[test]
fn ensure_writes_inbox_only_and_does_not_scan_index() {
    let sandbox = setup();
    let notes = sandbox.workbench_root().join("notes");
    fs::write(
        notes.join("index.json"),
        serde_json::to_string(&json!({
            "entries": {
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa": {
                    "common_path": "ai-software-dev/theme/a.md"
                }
            }
        }))
        .unwrap(),
    )
    .expect("index");

    let listed = list_notes_categories().expect("list");
    let ids: Vec<&str> = listed["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["inbox"]);
    assert!(notes.join("categories.json").is_file());
}

#[test]
fn add_and_delete_do_not_touch_index_or_raw() {
    let sandbox = setup();
    let notes = sandbox.workbench_root().join("notes");
    let id = "cccccccccccccccccccccccccccccccc";
    fs::create_dir_all(notes.join("raw/ai-software-dev/theme")).expect("dir");
    fs::write(notes.join("raw/ai-software-dev/theme/n.md"), "# n").expect("raw");
    fs::write(
        notes.join("index.json"),
        serde_json::to_string(&json!({
            "entries": { id: { "common_path": "ai-software-dev/theme/n.md" } }
        }))
        .unwrap(),
    )
    .expect("index");

    create_notes_category("research", "Research", "desc").expect("add");
    create_notes_category("ai-software-dev", "AI", "").expect("add existing slug");
    delete_notes_category("ai-software-dev").expect("delete");

    let index = fs::read_to_string(notes.join("index.json")).unwrap();
    assert!(index.contains("ai-software-dev/theme/n.md"));
    assert!(notes.join("raw/ai-software-dev/theme/n.md").is_file());
    assert!(!known_ids().unwrap().contains("ai-software-dev"));
    assert!(known_ids().unwrap().contains("research"));
}

#[test]
fn delete_inbox_and_invalid_id_rejected() {
    let _sandbox = setup();
    let err = delete_notes_category(INBOX_ID).expect_err("inbox");
    assert!(matches!(err, NotesCatError::BadRequest(_)));
    let err = create_notes_category("bad/id", "Bad", "").expect_err("slash");
    assert!(matches!(err, NotesCatError::BadRequest(_)));
}

#[test]
fn update_title_does_not_touch_index_or_id() {
    let sandbox = setup();
    let notes = sandbox.workbench_root().join("notes");
    let id = "dddddddddddddddddddddddddddddddd";
    fs::create_dir_all(notes.join("raw/ops/theme")).expect("dir");
    fs::write(notes.join("raw/ops/theme/n.md"), "# n").expect("raw");
    fs::write(
        notes.join("index.json"),
        serde_json::to_string(&json!({
            "entries": { id: { "common_path": "ops/theme/n.md" } }
        }))
        .unwrap(),
    )
    .expect("index");
    create_notes_category("ops", "Ops", "").expect("add");
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
    let index = fs::read_to_string(notes.join("index.json")).unwrap();
    assert!(index.contains("ops/theme/n.md"));
}

#[test]
fn duplicate_id_conflicts() {
    let _sandbox = setup();
    create_notes_category("ops", "Ops", "").expect("first");
    let err = create_notes_category("ops", "Ops 2", "").expect_err("dup");
    assert!(matches!(err, NotesCatError::Conflict(_)));
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
}
