use std::fs;

use serde_json::json;

use super::create_folder;
use crate::services::docs::{create_titled_doc, delete_doc};
use crate::services::spark_read::get_docs_index;
use crate::test_support::{default_inbox_id, default_workspace, default_workspace_dir, TestSandbox};

fn setup() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw");
    fs::create_dir_all(notes.join("digest")).expect("digest");
    sandbox
}

fn ws() -> String {
    default_workspace().id
}

fn inbox() -> String {
    default_inbox_id()
}

fn payload(extra: serde_json::Value) -> serde_json::Value {
    let mut body = json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "name": "design-notes",
    });
    if let Some(fields) = extra.as_object() {
        for (key, value) in fields {
            body[key] = value.clone();
        }
    }
    body
}

fn folder_dir(path: &str) -> std::path::PathBuf {
    default_workspace_dir().join("raw").join(inbox()).join(path)
}

#[test]
fn an_unknown_project_creates_no_directory() {
    let _sandbox = setup();
    let raw = default_workspace_dir().join("raw");
    let before = fs::read_dir(&raw).expect("raw").count();
    let v = create_folder(&payload(json!({ "project_id": "missing-project" })));
    assert!(v.get("ok").is_none(), "{v}");
    assert_eq!(fs::read_dir(&raw).expect("raw").count(), before);
    assert!(!listed("design-notes"));
}

#[test]
fn a_hidden_folder_stays_on_disk_and_leaves_the_index() {
    let _sandbox = setup();
    let made = create_folder(&payload(json!({ "name": ".git" })));
    assert_eq!(made["ok"], json!(true), "{made}");
    assert!(folder_dir(".git").is_dir());
    assert!(!listed(".git"));
}

fn listed(path: &str) -> bool {
    get_docs_index(Some(&ws()))["folders"]
        .as_array()
        .expect("folders")
        .iter()
        .any(|row| row["project_id"] == json!(inbox()) && row["path"] == json!(path))
}

#[test]
fn create_folder_at_the_project_root_is_still_listed_on_the_next_read() {
    let _sandbox = setup();
    let first = create_folder(&payload(json!({ "name": "design-notes" })));
    assert_eq!(first["ok"], json!(true), "{first}");
    assert_eq!(first["path"], json!("design-notes"));
    let second = create_folder(&payload(json!({ "name": "specialized-tech" })));
    assert_eq!(second["ok"], json!(true), "{second}");
    assert!(folder_dir("design-notes").is_dir());
    assert!(folder_dir("specialized-tech").is_dir());
    assert!(listed("design-notes"));
    assert!(listed("specialized-tech"));
}

#[test]
fn create_folder_makes_a_child_of_an_existing_folder() {
    let _sandbox = setup();
    let parent = create_folder(&payload(json!({ "name": "scope-functions" })));
    assert_eq!(parent["ok"], json!(true), "{parent}");
    let child = create_folder(&payload(json!({
        "parent": "scope-functions",
        "name": "design-notes",
    })));
    assert_eq!(child["path"], json!("scope-functions/design-notes"), "{child}");
    assert!(folder_dir("scope-functions/design-notes").is_dir());
    assert!(listed("scope-functions/design-notes"));
}

#[test]
fn create_folder_rejects_a_duplicate_or_illegal_name_without_changing_anything() {
    let _sandbox = setup();
    let made = create_titled_doc(&json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "title": "笔记",
        "source_type": "jot",
        "content": "正文",
        "folder": "scope-functions",
        "filename": "architecture-notes.md",
        "created_at": "202611101200",
    }));
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id");
    let file = crate::services::docs::doc_file_path(
        &default_workspace_dir(),
        "raw",
        &inbox(),
        "scope-functions/architecture-notes.md",
    )
    .expect("path");
    let before = fs::read_to_string(&file).expect("read");

    let again = create_folder(&payload(json!({ "name": "scope-functions" })));
    assert_eq!(again["_status"], json!(409), "{again}");
    assert_eq!(fs::read_to_string(&file).expect("read"), before);
    assert!(!listed("scope-functions"));

    let before_dirs = folder_dir("scope-functions").read_dir().expect("dir").count();
    for (name, parent) in [
        ("../secret", ""),
        ("a/b", ""),
        ("has:colon", ""),
        ("", ""),
        ("too-deep", "a/b/c"),
    ] {
        let mut body = payload(json!({}));
        if name.is_empty() {
            body.as_object_mut().expect("object").remove("name");
        } else {
            body["name"] = json!(name);
        }
        if !parent.is_empty() {
            body["parent"] = json!(parent);
        }
        let v = create_folder(&body);
        assert_eq!(v["_status"], json!(400), "{name} {parent}: {v}");
    }
    assert_eq!(folder_dir("scope-functions").read_dir().expect("dir").count(), before_dirs);
    assert_eq!(
        crate::services::spark_data::find_doc(&crate::services::spark_data::open().expect("db"), id)
            .expect("get")
            .expect("row")
            .doc
            .title
            .as_deref(),
        Some("笔记")
    );
}

#[test]
fn deleting_the_last_document_keeps_the_parent_folder_visible() {
    let _sandbox = setup();
    let made = create_titled_doc(&json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "title": "笔记",
        "source_type": "jot",
        "content": "正文",
        "folder": "scope-functions",
        "filename": "architecture-notes.md",
        "created_at": "202611101200",
    }));
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();
    let removed = delete_doc(&json!({ "id": id }));
    assert_eq!(removed["ok"], json!(true), "{removed}");
    assert!(folder_dir("scope-functions").is_dir());
    assert!(!folder_dir("scope-functions").join("architecture-notes.md").exists());
    assert!(listed("scope-functions"));
    let index = get_docs_index(Some(&ws()));
    assert!(index["entries"].get(&id).is_none(), "{index}");
}
