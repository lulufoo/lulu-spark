use std::fs;
use std::path::PathBuf;

use rusqlite::params;
use serde_json::json;

use super::move_folder;
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

fn raw(path: &str) -> PathBuf {
    default_workspace_dir().join("raw").join(inbox()).join(path)
}

fn digest(path: &str) -> PathBuf {
    default_workspace_dir().join("digest").join(inbox()).join(path)
}

fn folder(path: &str, parent: &str) -> serde_json::Value {
    let mut body = json!({ "workspace_id": ws(), "project_id": inbox(), "name": path });
    if !parent.is_empty() {
        body["parent"] = json!(parent);
    }
    create_folder(&body)
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

fn move_to(path: &str, target: &str) -> serde_json::Value {
    let mut body = json!({ "workspace_id": ws(), "project_id": inbox(), "path": path });
    if !target.is_empty() {
        body["target"] = json!(target);
    }
    move_folder(&body)
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

#[test]
fn move_folder_moves_the_subtree_under_the_target_folder() {
    let _sandbox = setup();
    assert_eq!(folder("plans", "")["ok"], json!(true), "create plans");
    assert_eq!(folder("2026", "plans")["ok"], json!(true), "create plans/2026");
    assert_eq!(folder("archive", "")["ok"], json!(true), "create archive");
    let made = note("plans", "notes.md", "笔记", "202611102220");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();
    let inner = note("plans/2026", "year.md", "年度", "202611102221");
    assert_eq!(inner["ok"], json!(true), "{inner}");
    let inner_id = inner["id"].as_str().expect("id").to_string();
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

    let moved = move_to("plans", "archive");
    assert_eq!(moved["ok"], json!(true), "{moved}");
    assert_eq!(moved["path"], json!("archive/plans"), "{moved}");
    assert_eq!(moved["moved"], json!(2), "{moved}");
    assert!(raw("archive/plans/notes.md").is_file());
    assert!(raw("archive/plans/notes-zh.md").is_file());
    assert!(raw("archive/plans/.attached").join(&id).join("pic.png").is_file());
    assert!(raw("archive/plans/2026/year.md").is_file());
    assert!(digest("archive/plans/notes.md").is_file());
    assert!(!raw("plans/notes.md").exists());
    assert!(!raw("plans/notes-zh.md").exists());
    // No shell is left behind: an empty `.attached` would block deleting the folder.
    assert!(!raw("plans/.attached").exists());
    assert!(!raw("plans/2026/year.md").exists());
    assert!(!digest("plans").exists());
    let after = find_doc(&open().expect("db"), &id).expect("find").expect("row").doc;
    assert_eq!(after.raw_path, "archive/plans/notes.md");
    assert_eq!(after.digest_path.as_deref(), Some("archive/plans/notes.md"));
    let inner_after = find_doc(&open().expect("db"), &inner_id).expect("find").expect("row").doc;
    assert_eq!(inner_after.raw_path, "archive/plans/2026/year.md");
    assert!(folder_row("archive/plans"));
    assert!(folder_row("archive/plans/2026"));
    assert!(!folder_row("plans"));
}

#[test]
fn move_folder_to_the_project_root_moves_one_level_up() {
    let sandbox = setup();
    assert_eq!(folder("a", "")["ok"], json!(true), "create a");
    assert_eq!(folder("b", "a")["ok"], json!(true), "create a/b");
    let made = note("a/b", "b.md", "笔记", "202611102225");
    assert_eq!(made["ok"], json!(true), "{made}");

    let moved = move_to("a/b", "");
    assert_eq!(moved["ok"], json!(true), "{moved}");
    assert_eq!(moved["path"], json!("b"), "{moved}");
    assert!(raw("b/b.md").is_file());
    assert!(!raw("a/b/b.md").exists());
    assert!(folder_row("b"));
    assert!(!folder_row("a/b"));
    assert_eq!(
        find_doc(&open().expect("db"), made["id"].as_str().expect("id"))
            .expect("find")
            .expect("row")
            .doc
            .raw_path,
        "b/b.md"
    );
}

#[test]
fn move_folder_to_its_parent_is_a_noop() {
    let sandbox = setup();
    assert_eq!(folder("a", "")["ok"], json!(true), "create a");
    assert_eq!(folder("b", "a")["ok"], json!(true), "create a/b");
    let made = note("a/b", "b.md", "笔记", "202611102226");
    assert_eq!(made["ok"], json!(true), "{made}");

    let moved = move_to("a/b", "a");
    assert_eq!(moved["ok"], json!(true), "{moved}");
    assert_eq!(moved["moved"], json!(0), "{moved}");
    assert!(raw_dir_unchanged(&sandbox));
    assert!(folder_row("a/b"));
}

fn raw_dir_unchanged(sandbox: &TestSandbox) -> bool {
    workspaces_dir(sandbox)
        .join(default_workspace().id)
        .join("raw")
        .join(inbox())
        .join("a/b/b.md")
        .is_file()
}

fn workspaces_dir(sandbox: &TestSandbox) -> PathBuf {
    sandbox.data_dir().join("workspaces")
}

#[test]
fn move_folder_refuses_subtree_targets_taken_paths_and_unknown_folders() {
    let sandbox = setup();
    assert_eq!(folder("plans", "")["ok"], json!(true), "create plans");
    assert_eq!(folder("inner", "plans")["ok"], json!(true), "create plans/inner");
    assert_eq!(folder("archive", "")["ok"], json!(true), "create archive");
    let made = note("plans", "notes.md", "笔记", "202611102227");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();

    let cycle = move_to("plans", "plans/inner");
    assert_eq!(cycle["_status"], json!(400), "{cycle}");
    assert!(raw("plans/notes.md").is_file());

    let unknown = move_to("plans", "ghost");
    assert_eq!(unknown["_status"], json!(400), "{unknown}");

    let missing = move_to("missing", "archive");
    assert_eq!(missing["_status"], json!(404), "{missing}");

    // A folder row at the destination blocks the move.
    assert_eq!(
        create_folder(&json!({ "workspace_id": ws(), "project_id": inbox(), "parent": "archive", "name": "plans" }))["ok"],
        json!(true)
    );
    let taken_folder = move_to("plans", "archive");
    assert_eq!(taken_folder["_status"], json!(409), "{taken_folder}");
    assert!(raw("plans/notes.md").is_file());
    assert!(folder_row("archive/plans"));

    // A bare directory on disk at the destination blocks it the same way.
    fs::remove_dir(raw("archive/plans")).expect("clear taken row dir");
    // The folder row still exists; drop it to exercise the disk-only branch.
    open()
        .expect("db")
        .execute("DELETE FROM folder WHERE project_id = ?1 AND path = ?2", params![inbox(), "archive/plans"])
        .expect("delete row");
    fs::create_dir_all(raw("archive/plans")).expect("recreate disk dir");
    fs::write(raw("archive/plans/.keep"), "占位").expect("keep");
    let taken_disk = move_to("plans", "archive");
    assert_eq!(taken_disk["_status"], json!(409), "{taken_disk}");
    assert!(raw("plans/notes.md").is_file());

    let self_moved = move_to("plans", "plans");
    assert_eq!(self_moved["ok"], json!(true), "{self_moved}");
    assert_eq!(self_moved["moved"], json!(0), "{self_moved}");
    assert!(raw("plans/notes.md").is_file());
}
