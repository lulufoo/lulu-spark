use std::fs;

use rusqlite::params;
use serde_json::json;

use super::move_doc_dir;
use crate::services::docs::{create_folder, create_titled_doc};
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

fn digest(path: &str) -> std::path::PathBuf {
    default_workspace_dir().join("digest").join(inbox()).join(path)
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

fn folder(path: &str, parent: &str) -> serde_json::Value {
    let mut payload = json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "name": path,
    });
    if !parent.is_empty() {
        payload["parent"] = json!(parent);
    }
    create_folder(&payload)
}

fn folder_row(path: &str) -> bool {
    open()
        .expect("db")
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM folder WHERE project_id = ?1 AND path = ?2)",
            params![inbox(), path],
            |row| row.get::<_, i64>(0),
        )
        .expect("folder row")
        == 1
}

fn move_to(id: &str, target: &str) -> serde_json::Value {
    let mut payload = json!({
        "workspace_id": ws(),
        "project_id": inbox(),
        "id": id,
    });
    if !target.is_empty() {
        payload["target"] = json!(target);
    }
    move_doc_dir(&payload)
}

#[test]
fn move_doc_dir_moves_the_doc_into_the_target_folder() {
    let _sandbox = setup();
    let made = note("plans", "notes.md", "笔记", "202611102220");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();
    assert_eq!(folder("archive", "")["ok"], json!(true), "create archive");
    assert_eq!(folder("2026", "archive")["ok"], json!(true), "create archive/2026");

    let conn = open().expect("db");
    let mut row = find_doc(&conn, &id).expect("find").expect("row").doc;
    row.translations = json!(["zh"]);
    row.digest_path = Some(row.raw_path.clone());
    assert!(update_doc(&conn, &row).expect("update"));
    fs::write(raw("plans/notes-zh.md"), "译文").expect("zh");
    let attached = raw("plans/.attached").join(&id);
    fs::create_dir_all(&attached).expect("attached");
    fs::write(attached.join("pic.png"), "图").expect("pic");
    fs::create_dir_all(digest("plans")).expect("digest dir");
    fs::write(digest("plans/notes.md"), "摘要").expect("digest");

    let moved = move_to(&id, "archive/2026");
    assert_eq!(moved["raw_path"], json!("archive/2026/notes.md"), "{moved}");
    assert!(raw("archive/2026/notes.md").is_file());
    assert!(raw("archive/2026/notes-zh.md").is_file());
    assert!(raw("archive/2026/.attached").join(&id).join("pic.png").is_file());
    assert!(!raw("plans/notes.md").exists());
    assert!(!raw("plans/notes-zh.md").exists());
    assert!(!raw("plans/.attached").join(&id).exists());
    assert!(digest("archive/2026/notes.md").is_file());
    assert!(!digest("plans/notes.md").exists());
    let after = find_doc(&open().expect("db"), &id).expect("find").expect("row").doc;
    assert_eq!(after.raw_path, "archive/2026/notes.md");
    assert_eq!(after.digest_path.as_deref(), Some("archive/2026/notes.md"));
    assert_eq!(after.title.as_deref(), Some("笔记"));
    // The folder the document left stays in the tree as an empty folder.
    assert!(folder_row("plans"));
    assert!(folder_row("archive/2026"));
}

#[test]
fn move_doc_dir_moves_back_to_the_project_root() {
    let _sandbox = setup();
    let made = note("plans", "notes.md", "笔记", "202611102221");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();

    let moved = move_to(&id, "");
    assert_eq!(moved["raw_path"], json!("notes.md"), "{moved}");
    assert!(raw("notes.md").is_file());
    assert!(!raw("plans/notes.md").exists());
    assert!(folder_row("plans"));
    let after = find_doc(&open().expect("db"), &id).expect("find").expect("row").doc;
    assert_eq!(after.raw_path, "notes.md");
}

#[test]
fn move_doc_dir_refuses_taken_or_invalid_targets() {
    let _sandbox = setup();
    let first = note("plans", "notes.md", "笔记", "202611102222");
    let second = note("archive", "notes.md", "重名", "202611102223");
    assert_eq!(first["ok"], json!(true), "{first}");
    assert_eq!(second["ok"], json!(true), "{second}");
    let id = first["id"].as_str().expect("id").to_string();

    let same = move_to(&id, "plans");
    assert_eq!(same["ok"], json!(true), "{same}");
    assert_eq!(same["raw_path"], json!("plans/notes.md"), "{same}");
    assert!(raw("plans/notes.md").is_file());

    let taken = move_to(&id, "archive");
    assert_eq!(taken["_status"], json!(409), "{taken}");
    assert!(raw("plans/notes.md").is_file());
    assert!(raw("archive/notes.md").is_file());

    let unknown = move_to(&id, "ghost");
    assert_eq!(unknown["_status"], json!(400), "{unknown}");
    assert!(raw("plans/notes.md").is_file());

    let missing = move_to("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "plans");
    assert_eq!(missing["_status"], json!(404), "{missing}");
    assert!(raw("plans/notes.md").is_file());
}
