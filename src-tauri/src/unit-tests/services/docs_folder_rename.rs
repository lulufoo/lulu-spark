use std::fs;

use serde_json::json;

use super::rename_folder;
use crate::services::docs::{create_folder, create_titled_doc};
use crate::services::spark_data::{find_doc, open, update_doc};
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

fn folder_dir(path: &str) -> std::path::PathBuf {
    default_workspace_dir().join("raw").join(inbox()).join(path)
}

fn listed(path: &str) -> bool {
    get_docs_index(Some(&ws()))["folders"]
        .as_array()
        .expect("folders")
        .iter()
        .any(|row| row["project_id"] == json!(inbox()) && row["path"] == json!(path))
}

fn rename(path: &str, name: &str) -> serde_json::Value {
    rename_folder(&json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "path": path,
        "name": name,
    }))
}

fn note(folder: &str, filename: &str, title: &str, at: &str) -> serde_json::Value {
    create_titled_doc(&json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "title": title,
        "source_type": "jot",
        "content": "正文",
        "folder": folder,
        "filename": filename,
        "created_at": at,
    }))
}

#[test]
fn rename_folder_moves_each_document_and_keeps_the_rest() {
    let _sandbox = setup();
    let made = note("scope-functions", "architecture-notes.md", "笔记", "202611101210");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();
    let nested = note("scope-functions/inner", "inner-notes.md", "子目录", "202611101211");
    assert_eq!(nested["ok"], json!(true), "{nested}");
    let outside = note("other", "outside.md", "外面", "202611101212");
    assert_eq!(outside["ok"], json!(true), "{outside}");
    assert_eq!(
        create_folder(&json!({
            "workspace_id": ws(),
            "project_id": inbox(),
            "parent": "scope-functions",
            "name": "empty",
        }))["ok"],
        json!(true)
    );

    let conn = open().expect("db");
    let mut row = find_doc(&conn, &id).expect("find").expect("row").doc;
    row.translations = json!(["zh"]);
    row.digest_path = Some(row.raw_path.clone());
    assert!(update_doc(&conn, &row).expect("update"));
    let raw = folder_dir("scope-functions");
    fs::write(raw.join("architecture-notes-zh.md"), "译文").expect("zh");
    fs::write(raw.join("notes.txt"), "保留").expect("extra");
    let attached = raw.join(".attached").join(&id);
    fs::create_dir_all(&attached).expect("attached");
    fs::write(attached.join("pic.png"), "图").expect("pic");
    let digest = default_workspace_dir().join("digest").join(inbox()).join("scope-functions");
    fs::create_dir_all(&digest).expect("digest dir");
    fs::write(digest.join("architecture-notes.md"), "摘要").expect("digest");

    let renamed = rename("scope-functions", "renamed");
    assert_eq!(renamed["path"], json!("renamed"), "{renamed}");
    assert!(folder_dir("renamed/architecture-notes.md").is_file());
    assert!(folder_dir("renamed/inner/inner-notes.md").is_file());
    assert!(folder_dir("renamed/architecture-notes-zh.md").is_file());
    assert!(folder_dir("renamed/.attached").join(&id).join("pic.png").is_file());
    assert!(folder_dir("renamed/notes.txt").is_file());
    assert!(folder_dir("renamed/empty").is_dir());
    assert!(!folder_dir("scope-functions").exists());
    assert!(folder_dir("other/outside.md").is_file());
    assert!(listed("renamed/empty"));
    assert!(!listed("scope-functions/empty"));
    let digest_now = default_workspace_dir().join("digest").join(inbox()).join("renamed/architecture-notes.md");
    assert!(digest_now.is_file());
    let after = find_doc(&open().expect("db"), &id).expect("find").expect("row").doc;
    assert_eq!(after.raw_path, "renamed/architecture-notes.md");
    assert_eq!(after.digest_path.as_deref(), Some("renamed/architecture-notes.md"));
    assert_eq!(after.title.as_deref(), Some("笔记"));
}

#[test]
fn rename_folder_moves_an_empty_folder_and_refuses_a_taken_name() {
    let _sandbox = setup();
    assert_eq!(
        create_folder(&json!({ "workspace_id": ws(), "project_id": inbox(), "name": "design-notes" }))["ok"],
        json!(true)
    );
    assert_eq!(
        create_folder(&json!({ "workspace_id": ws(), "project_id": inbox(), "name": "specialized-tech" }))["ok"],
        json!(true)
    );
    let same = rename("design-notes", "design-notes");
    assert_eq!(same["ok"], json!(true), "{same}");
    assert!(folder_dir("design-notes").is_dir());

    let blocked = rename("design-notes", "specialized-tech");
    assert_eq!(blocked["_status"], json!(409), "{blocked}");
    assert!(folder_dir("design-notes").is_dir());
    assert!(listed("design-notes"));
    assert!(listed("specialized-tech"));

    let renamed = rename("design-notes", "archive");
    assert_eq!(renamed["ok"], json!(true), "{renamed}");
    assert!(folder_dir("archive").is_dir());
    assert!(!folder_dir("design-notes").exists());
    assert!(listed("archive"));
    assert!(listed("specialized-tech"));

    let missing = rename("missing-folder", "archive");
    assert_eq!(missing["_status"], json!(404), "{missing}");
    let bad = rename("archive", "a/b");
    assert_eq!(bad["_status"], json!(400), "{bad}");
    assert!(folder_dir("archive").is_dir());
}
