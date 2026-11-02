//! The document's version number: which entries add 1, which never do, and how a pull sets it.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::docs::edit::edit_doc;
use crate::services::docs::store::write_markdown_atomic;
use crate::services::docs::{
    create_doc_content, create_project, delete_project, doc_file_path, move_doc, set_source_type,
    update_doc_content, workspace_root,
};
use crate::services::entry_write::save_entry;
use crate::services::spark_data::{self, find_doc, new_doc_id, set_doc_version};
use crate::test_support::{block_note_writes, TestSandbox};

const BODY: &str = "# 标题\n\n这是正文，足够写入。\n";
const NEW_BODY: &str = "# 标题\n\n这是改过的正文，足够写入。\n";
const DIGEST: &str = "# 标题 — 摘要\n\n> 创建时间：2026年6月19日 14:30\n\n## 概述\n\noverview";
const FRENCH: &str = "# Titre\n\nUn texte assez long.";

fn default_workspace_id() -> String {
    crate::test_support::default_workspace().id
}

/// Create a document in the default workspace; `extra` adds to or replaces the payload fields.
fn create(extra: Value) -> String {
    let mut payload = json!({
        "workspace_id": default_workspace_id(),
        "title": "T",
        "content": BODY,
        "digest": "never",
        "source_type": "summary",
    });
    for (key, value) in extra.as_object().expect("object") {
        payload[key] = value.clone();
    }
    let created = create_doc_content(&payload);
    assert_eq!(created["ok"], json!(true), "{created}");
    created["id"].as_str().expect("id").to_string()
}

fn version(id: &str) -> i64 {
    spark_data::get_doc(&spark_data::open().expect("db"), id).expect("get").expect("row").version
}

fn raw_file(id: &str) -> PathBuf {
    let found = find_doc(&spark_data::open().expect("db"), id).expect("find").expect("doc");
    let root = workspace_root(&found.workspace).expect("root");
    doc_file_path(&root, "raw", &found.doc.project_id, &found.doc.raw_path).expect("path")
}

fn update(id: &str, body: &str) -> Value {
    update_doc_content(&json!({ "id": id, "content": body, "digest": "never" }))
}

fn save(id: &str, layer: &str, content: &str, lang: Option<&str>) -> Value {
    save_entry(Path::new(""), layer.into(), id.into(), content.into(), lang.map(String::from))
}

fn new_project(name: &str) -> String {
    let made = create_project(&json!({ "workspace_id": default_workspace_id(), "project_name": name }));
    made["project_id"].as_str().expect("project").to_string()
}

#[test]
fn a_new_document_is_at_version_zero_whatever_files_it_is_created_with() {
    let _sandbox = TestSandbox::new();
    let plain = create(json!({}));
    let rich = create(json!({
        "digest": "always",
        "digest_body": DIGEST,
        "translations": [{ "lang": "fr", "content": FRENCH }],
    }));
    assert_eq!((version(&plain), version(&rich)), (0, 0));
}

#[test]
fn updating_the_body_adds_one_each_time() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({}));

    assert_eq!(update(&id, NEW_BODY)["ok"], json!(true));
    assert_eq!(version(&id), 1);
    assert_eq!(update(&id, BODY)["ok"], json!(true));
    assert_eq!(version(&id), 2);
}

#[test]
fn updating_the_body_and_the_digest_in_one_call_adds_one_not_two() {
    let _sandbox = TestSandbox::new();
    let with_digest = create(json!({ "digest": "always", "digest_body": DIGEST }));
    let without_digest = create(json!({}));
    let payload = |id: &str| {
        json!({ "id": id, "content": NEW_BODY, "digest": "always", "digest_body": DIGEST })
    };

    let first = update_doc_content(&payload(&with_digest));
    let second = update_doc_content(&payload(&without_digest)); // this one gains its digest row entry
    assert_eq!((&first["ok"], first["digest_path"].is_string()), (&json!(true), true), "{first}");
    assert_eq!((&second["ok"], second["digest_path"].is_string()), (&json!(true), true), "{second}");
    assert_eq!((version(&with_digest), version(&without_digest)), (1, 1));
}

#[test]
fn an_update_that_is_refused_leaves_the_version() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({}));

    assert_eq!(update(&id, "  ")["_status"], json!(400), "empty body");
    assert_eq!(update(&new_doc_id(), NEW_BODY)["_status"], json!(404), "no such document");
    fs::remove_file(raw_file(&id)).expect("remove");
    assert_eq!(update(&id, NEW_BODY)["_status"], json!(404), "file gone");

    assert_eq!(version(&id), 0);
}

#[test]
fn saving_in_the_editor_adds_one_for_the_body_the_digest_and_a_translation() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({
        "digest": "always",
        "digest_body": DIGEST,
        "translations": [{ "lang": "fr", "content": FRENCH }],
    }));

    for (n, (layer, lang)) in [("raw", None), ("digest", None), ("raw", Some("fr"))].into_iter().enumerate() {
        assert_eq!(save(&id, layer, "# changed\n\ntext", lang)["ok"], json!(true), "{layer} {lang:?}");
        assert_eq!(version(&id), n as i64 + 1, "after saving {layer} {lang:?}");
    }
}

#[test]
fn a_save_that_is_refused_leaves_the_version() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({}));

    assert_eq!(save(&id, "digest", "x", None)["_status"], json!(404), "no digest file");
    assert_eq!(save(&id, "raw", "x", Some("fr"))["_status"], json!(404), "no such translation file");
    assert_eq!(save(&id, "other", "x", None)["_status"], json!(400), "not an editable layer");

    assert_eq!(version(&id), 0);
}

#[test]
fn if_the_version_step_fails_an_update_puts_the_old_body_back() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({}));
    block_note_writes("UPDATE");

    let failed = update(&id, NEW_BODY);

    assert_eq!(failed["_status"], json!(500), "{failed}");
    assert_eq!(fs::read_to_string(raw_file(&id)).expect("read"), BODY);
    assert_eq!(version(&id), 0);
}

#[test]
fn if_the_version_step_fails_a_save_reports_the_error() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({}));
    block_note_writes("UPDATE");

    let failed = save(&id, "raw", "# changed\n\ntext", None);

    assert_eq!(failed["_status"], json!(500), "{failed}");
    assert_eq!(version(&id), 0);
}

#[test]
fn changing_properties_does_not_add_one_and_does_not_take_a_number_back() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({}));
    update(&id, NEW_BODY);
    update(&id, BODY);
    assert_eq!(version(&id), 2);

    let edited = edit_doc(&id, |_, doc| {
        doc.done = true;
        doc.importance = Some("high".into());
        doc.title = Some("New".into());
        Ok(json!({ "ok": true }))
    });
    assert_eq!(edited["ok"], json!(true), "{edited}");
    assert_eq!(set_source_type(&id, "dialogue")["ok"], json!(true));

    assert_eq!(version(&id), 2);
}

#[test]
fn moving_a_document_does_not_add_one() {
    let _sandbox = TestSandbox::new();
    let shelf = new_project("Shelf");
    let id = create(json!({}));
    update(&id, NEW_BODY);

    let moved = move_doc(&json!({ "id": id, "project_id": shelf }));

    assert_eq!(moved["ok"], json!(true), "{moved}");
    assert_eq!(version(&id), 1);
}

#[test]
fn deleting_a_project_moves_its_documents_without_adding_one() {
    let _sandbox = TestSandbox::new();
    let project = new_project("Plans");
    let id = create(json!({ "project_id": project }));
    update(&id, NEW_BODY);

    let done = delete_project(&json!({ "workspace_id": default_workspace_id(), "project_id": project }));

    assert_eq!(done["ok"], json!(true), "{done}");
    assert_eq!(version(&id), 1);
}

#[test]
fn a_pull_writes_files_without_adding_one_and_the_cloud_number_is_then_set() {
    let _sandbox = TestSandbox::new();
    let id = create(json!({}));
    update(&id, NEW_BODY);

    write_markdown_atomic(&raw_file(&id), "# from the cloud\n\ntext").expect("write");
    assert_eq!(version(&id), 1, "writing the file alone changes nothing");

    assert!(set_doc_version(&spark_data::open().expect("db"), &id, 9).expect("set"));
    assert_eq!(version(&id), 9);
    assert_eq!(update(&id, BODY)["ok"], json!(true));
    assert_eq!(version(&id), 10, "the next edit goes on from the cloud's number");
}
