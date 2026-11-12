use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::docs::{
    create_doc, create_doc_content, create_folder, create_project, create_titled_doc,
    create_workspace, disable_workspace, move_doc, search,
};
use crate::services::spark_data;
use crate::test_support::TestSandbox;

const BODY: &str = "# 标题\n\n这是正文，足够写入。\n";
const PNG: &[u8] = b"\x89PNG\r\n";
const WORD: &str = "迁仓关键词";

fn setup() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw");
    fs::create_dir_all(notes.join("digest")).expect("digest");
    sandbox
}

fn default_id() -> String {
    crate::test_support::default_workspace().id
}

fn new_workspace(name: &str) -> String {
    create_workspace(&json!({ "workspace_name": name }))["workspace_id"]
        .as_str()
        .expect("workspace_id")
        .to_string()
}

fn new_project(workspace: &str, title: &str) -> String {
    create_project(&json!({ "workspace_id": workspace, "project_name": title }))["project_id"]
        .as_str()
        .expect("project_id")
        .to_string()
}

fn inbox_of(workspace: &str) -> String {
    spark_data::get_default_project(&spark_data::open().expect("db"), workspace)
        .expect("lookup")
        .expect("inbox")
        .id
}

fn workspaces_dir(sandbox: &TestSandbox) -> PathBuf {
    sandbox.data_dir().join("workspaces")
}

fn write(workspace: &str, project: Option<&str>, body: &str) -> serde_json::Value {
    let mut payload = json!({
        "workspace_id": workspace,
        "title": "T",
        "content": body,
        "digest": "never",
        "source_type": "summary",
    });
    if let Some(p) = project {
        payload["project_id"] = json!(p);
    }
    create_doc_content(&payload)
}

fn stage_bytes(sandbox: &TestSandbox, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage");
    let p = dir.join(name);
    fs::write(&p, bytes).expect("write");
    p.canonicalize().expect("canon")
}

#[test]
fn move_doc_cross_workspace_moves_attachments_and_the_body() {
    let sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_proj = new_project(&dest_ws, "Shelf");
    let png = stage_bytes(&sandbox, "pic.png", PNG);
    let source = stage_bytes(
        &sandbox,
        "source.md",
        format!("{BODY}\n![x](pic.png)\n").as_bytes(),
    );
    let created = create_doc(&json!({
            "workspace_id": default_id(),
            "source_path": source.to_str().unwrap(),
            "title": "T",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": [png.to_str().unwrap()],
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_proj,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("ok"), Some(&json!(true)), "{moved}");
    assert_eq!(moved["workspace_id"], json!(dest_ws));
    assert_eq!(moved["project_id"], json!(dest_proj));
    let rel = spark_data::get_doc(&spark_data::open().expect("db"), id)
        .expect("get")
        .expect("row")
        .raw_path;
    let dest_file = workspaces_dir(&sandbox)
        .join(&dest_ws)
        .join("raw")
        .join(&dest_proj)
        .join(&rel);
    assert!(dest_file.is_file(), "{}", dest_file.display());
    let attached = dest_file.parent().unwrap().join(".attached").join(id).join("pic.png");
    assert!(attached.is_file(), "{}", attached.display());
    let src_root = workspaces_dir(&sandbox).join(default_id()).join("raw");
    let leftover = walkdir_has(&src_root, id);
    assert!(!leftover, "source workspace still holds {id}");
}

#[test]
fn move_doc_renames_when_the_dest_path_is_taken() {
    let sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_proj = new_project(&dest_ws, "Shelf");
    let created = write(&default_id(), None, BODY);
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let rel = spark_data::get_doc(&spark_data::open().expect("db"), id)
        .expect("get")
        .expect("row")
        .raw_path;
    let taken = workspaces_dir(&sandbox)
        .join(&dest_ws)
        .join("raw")
        .join(&dest_proj)
        .join(&rel);
    fs::create_dir_all(taken.parent().unwrap()).expect("parent");
    fs::write(&taken, b"occupied").expect("taken");
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_proj,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("ok"), Some(&json!(true)), "{moved}");
    let new_rel = spark_data::get_doc(&spark_data::open().expect("db"), id)
        .expect("get")
        .expect("row")
        .raw_path;
    assert_ne!(new_rel, rel);
    assert_eq!(fs::read(&taken).unwrap(), b"occupied");
}

#[test]
fn move_doc_refuses_a_disabled_target_workspace() {
    let _sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_proj = new_project(&dest_ws, "Shelf");
    disable_workspace(&json!({ "workspace_id": dest_ws }));
    let created = write(&default_id(), None, BODY);
    let id = created["id"].as_str().unwrap();
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_proj,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("_status"), Some(&json!(409)), "{moved}");
}

fn walkdir_has(root: &std::path::Path, needle: &str) -> bool {
    fn walk(dir: &std::path::Path, needle: &str) -> bool {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == needle)
            {
                return true;
            }
            if path.is_dir() && walk(&path, needle) {
                return true;
            }
        }
        false
    }
    walk(root, needle)
}

#[test]
fn move_doc_reindexes_only_the_target_workspace() {
    let _sandbox = setup();
    let dest_ws = new_workspace("Dest");
    let dest_inbox = inbox_of(&dest_ws);
    let created = write(&default_id(), None, &format!("{BODY}\n{WORD}\n"));
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap().to_string();
    crate::services::index_build::rebuild_spark_index(&crate::config::paths::repo_root().expect("root"))
        .expect("index");
    let moved = move_doc(&json!({
            "id": id,
            "project_id": dest_inbox,
            "target_workspace_id": dest_ws,
        }),
    );
    assert_eq!(moved.get("ok"), Some(&json!(true)), "{moved}");
    let dest_hits = search(&json!({ "q": WORD, "workspace_id": dest_ws }));
    let dest_ids: Vec<String> = dest_hits["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(dest_ids, vec![id.clone()], "{dest_hits}");
    let src_hits = search(&json!({ "q": WORD, "workspace_id": default_id() }));
    let src_ids: Vec<String> = src_hits["items"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|i| i["id"].as_str().map(str::to_string))
        .collect();
    assert!(!src_ids.contains(&id), "{src_hits}");
}

fn raw_dir(sandbox: &TestSandbox, ws: &str, project: &str) -> PathBuf {
    workspaces_dir(sandbox).join(ws).join("raw").join(project)
}

fn digest_dir(sandbox: &TestSandbox, ws: &str, project: &str) -> PathBuf {
    workspaces_dir(sandbox).join(ws).join("digest").join(project)
}

fn move_within(id: &str, project: &str, target: &str) -> serde_json::Value {
    let mut body = json!({
        "id": id,
        "project_id": project,
        "workspace_id": default_id(),
    });
    if !target.is_empty() {
        body["target"] = json!(target);
    }
    move_doc(&body)
}

fn titled_note(folder: &str, filename: &str, title: &str, at: &str) -> serde_json::Value {
    create_titled_doc(&json!({
        "workspace_id": default_id(),
        "project_id": inbox_of(&default_id()),
        "title": title,
        "source_type": "jot",
        "content": "正文",
        "folder": folder,
        "filename": filename,
        "created_at": at,
    }))
}

#[test]
fn move_doc_within_a_project_moves_body_attached_and_digest_into_the_target_folder() {
    let sandbox = setup();
    let ws = default_id();
    let proj = inbox_of(&ws);
    assert_eq!(
        create_folder(&json!({ "workspace_id": ws, "project_id": proj, "name": "archive" }))["ok"],
        json!(true)
    );
    assert_eq!(
        create_folder(&json!({ "workspace_id": ws, "project_id": proj, "parent": "archive", "name": "2026" }))["ok"],
        json!(true)
    );
    let made = titled_note("plans", "notes.md", "笔记", "202611102220");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();
    let conn = spark_data::open().expect("db");
    let mut row = spark_data::find_doc(&conn, &id).expect("find").expect("row").doc;
    row.translations = json!(["zh"]);
    row.digest_path = Some(row.raw_path.clone());
    assert!(spark_data::update_doc(&conn, &row).expect("update"));
    let raw = raw_dir(&sandbox, &ws, &proj);
    let digest = digest_dir(&sandbox, &ws, &proj);
    fs::write(raw.join("plans/notes-zh.md"), "译文").expect("zh");
    let attached = raw.join("plans/.attached").join(&id);
    fs::create_dir_all(&attached).expect("attached");
    fs::write(attached.join("pic.png"), "图").expect("pic");
    fs::create_dir_all(digest.join("plans")).expect("digest dir");
    fs::write(digest.join("plans/notes.md"), "摘要").expect("digest");

    let moved = move_within(&id, &proj, "archive/2026");
    assert_eq!(moved["ok"], json!(true), "{moved}");
    assert_eq!(moved["raw_path"], json!("archive/2026/notes.md"), "{moved}");
    assert!(raw.join("archive/2026/notes.md").is_file());
    assert!(raw.join("archive/2026/notes-zh.md").is_file());
    assert!(raw.join("archive/2026/.attached").join(&id).join("pic.png").is_file());
    assert!(!raw.join("plans/notes.md").exists());
    assert!(!raw.join("plans/notes-zh.md").exists());
    // No shell is left behind: an empty `.attached` would block deleting the folder.
    assert!(!raw.join("plans/.attached").exists());
    assert!(digest.join("archive/2026/notes.md").is_file());
    assert!(!digest.join("plans").exists());
    let after = spark_data::find_doc(&spark_data::open().expect("db"), &id)
        .expect("find")
        .expect("row")
        .doc;
    assert_eq!(after.raw_path, "archive/2026/notes.md");
    assert_eq!(after.digest_path.as_deref(), Some("archive/2026/notes.md"));
}

#[test]
fn move_doc_within_a_project_moves_back_to_the_project_root() {
    let sandbox = setup();
    let ws = default_id();
    let proj = inbox_of(&ws);
    let made = titled_note("plans", "notes.md", "笔记", "202611102221");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();

    let moved = move_within(&id, &proj, "");
    assert_eq!(moved["ok"], json!(true), "{moved}");
    assert_eq!(moved["raw_path"], json!("notes.md"), "{moved}");
    assert!(raw_dir(&sandbox, &ws, &proj).join("notes.md").is_file());
    assert!(!raw_dir(&sandbox, &ws, &proj).join("plans/notes.md").exists());
}

#[test]
fn move_doc_within_a_project_keeps_the_path_when_the_folder_is_unchanged() {
    let sandbox = setup();
    let ws = default_id();
    let proj = inbox_of(&ws);
    let made = titled_note("plans", "notes.md", "笔记", "202611102222");
    assert_eq!(made["ok"], json!(true), "{made}");
    let id = made["id"].as_str().expect("id").to_string();

    let moved = move_within(&id, &proj, "plans");
    assert_eq!(moved["ok"], json!(true), "{moved}");
    assert_eq!(moved["raw_path"], json!("plans/notes.md"), "{moved}");
    assert!(raw_dir(&sandbox, &ws, &proj).join("plans/notes.md").is_file());
}

#[test]
fn move_doc_within_a_project_refuses_taken_targets_and_unknown_folders() {
    let sandbox = setup();
    let ws = default_id();
    let proj = inbox_of(&ws);
    let first = titled_note("plans", "notes.md", "笔记", "202611102223");
    assert_eq!(first["ok"], json!(true), "{first}");
    let id = first["id"].as_str().expect("id").to_string();
    let second = titled_note("archive", "notes.md", "重名", "202611102224");
    assert_eq!(second["ok"], json!(true), "{second}");

    let taken = move_within(&id, &proj, "archive");
    assert_eq!(taken["_status"], json!(409), "{taken}");
    assert!(raw_dir(&sandbox, &ws, &proj).join("plans/notes.md").is_file());
    assert!(raw_dir(&sandbox, &ws, &proj).join("archive/notes.md").is_file());

    let unknown = move_within(&id, &proj, "ghost");
    assert_eq!(unknown["_status"], json!(400), "{unknown}");
    assert!(raw_dir(&sandbox, &ws, &proj).join("plans/notes.md").is_file());

    let missing = move_within("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", &proj, "plans");
    assert_eq!(missing["_status"], json!(404), "{missing}");
    assert!(raw_dir(&sandbox, &ws, &proj).join("plans/notes.md").is_file());
}
