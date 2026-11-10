use std::fs;

use serde_json::json;

use super::rename_doc;
use crate::services::docs::create_titled_doc;
use crate::services::spark_data::{find_doc, open, update_doc};
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

fn raw(path: &str) -> std::path::PathBuf {
    default_workspace_dir().join("raw").join(inbox()).join(path)
}

fn rename(id: &str, name: &str) -> serde_json::Value {
    rename_doc(&json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "id": id,
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
fn rename_doc_moves_the_file_and_keeps_the_title() {
    let _sandbox = setup();
    let made = note("scope-functions", "architecture-notes.md", "笔记", "202611102210");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();
    let other = note("scope-functions", "inner-notes.md", "子目录", "202611102211");
    assert_eq!(other["ok"], json!(true), "{other}");

    let conn = open().expect("db");
    let mut row = find_doc(&conn, &id).expect("find").expect("row").doc;
    row.translations = json!(["zh"]);
    row.digest_path = Some(row.raw_path.clone());
    assert!(update_doc(&conn, &row).expect("update"));
    let dir = raw("scope-functions");
    fs::write(dir.join("architecture-notes-zh.md"), "译文").expect("zh");
    let attached = dir.join(".attached").join(&id);
    fs::create_dir_all(&attached).expect("attached");
    fs::write(attached.join("pic.png"), "图").expect("pic");
    let digest = default_workspace_dir()
        .join("digest")
        .join(inbox())
        .join("scope-functions");
    fs::create_dir_all(&digest).expect("digest dir");
    fs::write(digest.join("architecture-notes.md"), "摘要").expect("digest");

    let renamed = rename(&id, "archive.md");
    assert_eq!(renamed["raw_path"], json!("scope-functions/archive.md"), "{renamed}");
    assert!(raw("scope-functions/archive.md").is_file());
    assert!(raw("scope-functions/archive-zh.md").is_file());
    assert!(raw("scope-functions/.attached").join(&id).join("pic.png").is_file());
    assert!(raw("scope-functions/inner-notes.md").is_file());
    assert!(!raw("scope-functions/architecture-notes.md").exists());
    assert!(!raw("scope-functions/architecture-notes-zh.md").exists());
    let digest_now = default_workspace_dir()
        .join("digest")
        .join(inbox())
        .join("scope-functions/archive.md");
    assert!(digest_now.is_file());
    let after = find_doc(&open().expect("db"), &id).expect("find").expect("row").doc;
    assert_eq!(after.raw_path, "scope-functions/archive.md");
    assert_eq!(after.digest_path.as_deref(), Some("scope-functions/archive.md"));
    assert_eq!(after.title.as_deref(), Some("笔记"));
}

#[test]
fn rename_doc_refuses_a_taken_or_invalid_name() {
    let _sandbox = setup();
    let first = note("plans", "notes.md", "笔记", "202611102212");
    let second = note("plans", "other.md", "外面", "202611102213");
    assert_eq!(first["ok"], json!(true), "{first}");
    assert_eq!(second["ok"], json!(true), "{second}");
    let id = first["id"].as_str().expect("id").to_string();

    let same = rename(&id, "notes.md");
    assert_eq!(same["ok"], json!(true), "{same}");
    assert!(raw("plans/notes.md").is_file());

    let taken = rename(&id, "other.md");
    assert_eq!(taken["_status"], json!(409), "{taken}");
    assert!(raw("plans/notes.md").is_file());
    assert!(raw("plans/other.md").is_file());

    let bad = rename(&id, "a/b.md");
    assert_eq!(bad["_status"], json!(400), "{bad}");
    assert!(raw("plans/notes.md").is_file());

    let missing = rename("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "archive.md");
    assert_eq!(missing["_status"], json!(404), "{missing}");
}
