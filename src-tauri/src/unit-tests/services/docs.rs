use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use super::*;
use crate::services::spark_data::{self, count_docs_in_project, get_workspace};
use crate::test_support::TestSandbox;

const BODY: &str = "# 标题\n\n这是正文，足够写入。\n";

fn ok(value: &Value) -> &Value {
    assert_eq!(value.get("ok"), Some(&json!(true)), "expected ok, got {value}");
    value
}

fn status(value: &Value) -> u64 {
    value.get("_status").and_then(Value::as_u64).unwrap_or_else(|| panic!("expected an error, got {value}"))
}

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_else(|| panic!("missing {key} in {value}")).to_string()
}

fn new_workspace(name: &str) -> String {
    let made = create_workspace(&json!({ "name": name }));
    text(&ok(&made)["workspace"], "id")
}

fn write(workspace: Option<&str>, project: Option<&str>) -> Value {
    let mut payload = json!({ "title": "T", "content": BODY, "digest": "never" });
    if let Some(w) = workspace {
        payload["workspace"] = json!(w);
    }
    if let Some(p) = project {
        payload["project"] = json!(p);
    }
    create_doc_content(&payload)
}

fn workspaces_dir(sandbox: &TestSandbox) -> PathBuf {
    sandbox.data_dir().join("workspaces")
}

#[test]
fn a_new_workspace_is_writable_at_once_and_files_land_in_its_folder() {
    let sandbox = TestSandbox::new();
    let ws = new_workspace("AI memory");
    let created = write(Some(&ws), None);
    let id = text(ok(&created), "id");
    assert_eq!((text(&created, "workspace"), text(&created, "project")), (ws.clone(), "inbox".to_string()));

    let on_disk = fs::read_dir(workspaces_dir(&sandbox).join(&ws).join("raw/inbox/notes")).expect("dir");
    assert_eq!(on_disk.count(), 1);
    let read = get_file(&json!({ "workspace": ws, "id": id }));
    assert_eq!(read["content"], json!(BODY));

    let conn = spark_data::open().expect("db");
    let table = get_workspace(&conn, &ws).expect("get").expect("row").doc_table;
    assert_eq!(table, format!("doc_{ws}"));
    assert!(spark_data::get_doc(&conn, &table, &id).expect("get").is_some());
}

#[test]
fn omitting_workspace_and_project_writes_to_default_inbox() {
    let sandbox = TestSandbox::new();
    let created = write(None, None);
    let id = text(ok(&created), "id");
    assert_eq!((text(&created, "workspace"), text(&created, "project")), ("default".to_string(), "inbox".to_string()));
    assert!(workspaces_dir(&sandbox).join("default/raw/inbox/notes").is_dir());
    assert_eq!(get_file(&json!({ "id": id }))["content"], json!(BODY));
}

#[test]
fn a_project_of_another_workspace_is_rejected() {
    let _sandbox = TestSandbox::new();
    ok(&create_project(&json!({ "folder": "alpha", "title": "Alpha" })));
    let other = new_workspace("Other");

    assert_eq!(status(&write(Some(&other), Some("alpha"))), 400, "alpha is a default project");
    ok(&write(None, Some("alpha")));
    assert_eq!(status(&write(None, Some("nope"))), 400);
    assert_eq!(status(&write(Some("missing"), None)), 404);
}

#[test]
fn a_document_is_only_found_in_its_own_workspace() {
    let _sandbox = TestSandbox::new();
    let other = new_workspace("Other");
    let id = text(ok(&write(None, None)), "id");

    assert_eq!(status(&get_file(&json!({ "workspace": other, "id": id }))), 404);
    let update = update_doc_content(&json!({ "workspace": other, "id": id, "content": BODY, "digest": "never" }));
    assert_eq!(status(&update), 404);
    assert_eq!(status(&delete_doc(&json!({ "workspace": other, "id": id }))), 404);
    ok(&delete_doc(&json!({ "id": id })));
}

#[test]
fn update_overwrites_the_body_and_refuses_create_only_fields() {
    let _sandbox = TestSandbox::new();
    let id = text(ok(&write(None, None)), "id");
    let newer = "# 新标题\n\n更新后的正文。\n";
    ok(&update_doc_content(&json!({ "id": id, "content": newer, "digest": "never" })));
    assert_eq!(get_file(&json!({ "id": id }))["content"], json!(newer));

    for key in ["title", "project", "folder", "theme", "created_at", "source_type"] {
        let mut payload = json!({ "id": id, "content": newer, "digest": "never" });
        payload[key] = json!("x");
        assert_eq!(status(&update_doc_content(&payload)), 400, "{key}");
    }
}

#[test]
fn a_digest_is_written_and_read_back() {
    let _sandbox = TestSandbox::new();
    let created = create_doc_content(&json!({
        "title": "T", "content": BODY, "digest": "always", "digest_body": "# 摘要\n\n要点。\n"
    }));
    let id = text(ok(&created), "id");
    // The digest body is trimmed on the way in, as it always was.
    assert_eq!(get_digest(&json!({ "id": id }))["content"], json!("# 摘要\n\n要点。"));
    assert_eq!(get_file(&json!({ "id": id, "layer": "bogus" }))["_status"], json!(400));
}

#[test]
fn delete_doc_removes_files_and_row() {
    let sandbox = TestSandbox::new();
    let created = create_doc_content(&json!({
        "title": "T", "content": BODY, "digest": "always", "digest_body": "# 摘要\n\n要点。\n"
    }));
    let id = text(ok(&created), "id");
    let removed = delete_doc(&json!({ "id": id }));
    assert_eq!(ok(&removed)["removed"], json!(2));
    assert_eq!(status(&get_file(&json!({ "id": id }))), 404);

    let leftover = fs::read_dir(workspaces_dir(&sandbox).join("default/raw/inbox/notes")).expect("dir").count();
    assert_eq!(leftover, 0);
}

#[test]
fn default_workspace_cannot_be_deleted_and_the_default_can_be_renamed() {
    let _sandbox = TestSandbox::new();
    assert_eq!(status(&delete_workspace(&json!({ "id": "default" }))), 400);
    let renamed = rename_workspace(&json!({ "id": "default", "name": "Home" }));
    assert_eq!(ok(&renamed)["workspace"]["name"], json!("Home"));
    assert_eq!(ok(&renamed)["workspace"]["id"], json!("default"));
    let listed = list_workspaces();
    assert_eq!(listed["workspaces"][0]["id"], json!("default"), "default comes first");
}

#[test]
fn a_workspace_with_documents_is_kept_and_an_empty_one_goes_with_its_folder() {
    let sandbox = TestSandbox::new();
    let ws = new_workspace("Temp");
    let id = text(ok(&write(Some(&ws), None)), "id");

    assert_eq!(status(&delete_workspace(&json!({ "id": ws }))), 409);
    ok(&delete_doc(&json!({ "workspace": ws, "id": id })));
    ok(&delete_workspace(&json!({ "id": ws })));

    let conn = spark_data::open().expect("db");
    assert!(get_workspace(&conn, &ws).expect("get").is_none());
    assert!(!workspaces_dir(&sandbox).join(&ws).exists());
    assert_eq!(status(&delete_workspace(&json!({ "id": ws }))), 404);
}

#[test]
fn workspace_names_and_view_modes_are_checked() {
    let _sandbox = TestSandbox::new();
    let made = create_workspace(&json!({ "name": "Notes", "view_mode": "timeline" }));
    assert_eq!(ok(&made)["workspace"]["view_mode"], json!("timeline"));
    assert_eq!(ok(&made)["workspace"]["is_default"], json!(false));
    assert_eq!(status(&create_workspace(&json!({ "name": "Notes" }))), 409, "name is unique");
    assert_eq!(status(&create_workspace(&json!({ "name": "  " }))), 400);
    assert_eq!(status(&create_workspace(&json!({ "name": "X", "view_mode": "grid" }))), 400);

    let ws = text(&made["workspace"], "id");
    let switched = set_view_mode(&json!({ "id": ws, "view_mode": "directory" }));
    assert_eq!(ok(&switched)["workspace"]["view_mode"], json!("directory"));
    assert_eq!(status(&set_view_mode(&json!({ "id": ws }))), 400);
}

#[test]
fn project_crud_follows_the_workspace_and_protects_inbox_and_documents() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Work");
    let made = create_project(&json!({ "workspace": ws, "folder": "plans", "title": "Plans", "description": "d" }));
    assert_eq!(ok(&made)["project"]["folder"], json!("plans"));
    assert_eq!(status(&create_project(&json!({ "workspace": ws, "folder": "plans", "title": "Again" }))), 409);
    assert_eq!(status(&create_project(&json!({ "workspace": ws, "folder": "a/b", "title": "Bad" }))), 400);
    assert_eq!(status(&create_project(&json!({ "workspace": ws, "title": " " }))), 400);

    let minted = create_project(&json!({ "workspace": ws, "title": "Auto" }));
    assert_eq!(ok(&minted)["project"]["folder"], ok(&minted)["project"]["id"], "folder defaults to the id");

    let listed = list_projects(&json!({ "workspace": ws }));
    let folders: Vec<_> = listed["projects"].as_array().expect("array").iter().map(|p| text(p, "folder")).collect();
    assert!(folders.contains(&"inbox".to_string()) && folders.contains(&"plans".to_string()), "{folders:?}");

    let updated = update_project(&json!({ "workspace": ws, "project": "plans", "title": "Roadmaps" }));
    assert_eq!((text(&ok(&updated)["project"], "title"), text(&updated["project"], "folder")), ("Roadmaps".into(), "plans".into()));
    assert_eq!(status(&update_project(&json!({ "workspace": ws, "project": "ghost", "title": "x" }))), 400);

    assert_eq!(status(&delete_project(&json!({ "workspace": ws, "project": "inbox" }))), 400);
    let id = text(ok(&write(Some(&ws), Some("plans"))), "id");
    assert_eq!(status(&delete_project(&json!({ "workspace": ws, "project": "plans" }))), 409);
    ok(&delete_doc(&json!({ "workspace": ws, "id": id })));
    ok(&delete_project(&json!({ "workspace": ws, "project": "plans" })));

    let conn = spark_data::open().expect("db");
    let table = get_workspace(&conn, &ws).expect("get").expect("row").doc_table;
    assert_eq!(count_docs_in_project(&conn, &table, "none").expect("count"), 0);
}

#[test]
fn export_file_copies_the_raw_file() {
    let sandbox = TestSandbox::new();
    let id = text(ok(&write(None, None)), "id");
    let dest = sandbox.cache_dir().join("exported");
    let copied = export_file(&json!({ "id": id, "dest_dir": dest.to_str().expect("utf8") }));
    let path = text(ok(&copied), "path");
    assert_eq!(fs::read_to_string(path).expect("copy"), BODY);
    assert_eq!(status(&export_file(&json!({ "id": id }))), 400);
}

#[test]
fn get_path_points_at_the_raw_file_without_its_body() {
    let sandbox = TestSandbox::new();
    let created = write(None, None);
    let id = text(ok(&created), "id");
    let got = get_path(&json!({ "id": id }));
    let path = PathBuf::from(text(ok(&got), "path"));
    let expected = workspaces_dir(&sandbox).join("default/raw/inbox").canonicalize().expect("dir");
    assert!(path.canonicalize().expect("file").starts_with(expected), "{path:?}");
    assert_eq!(fs::read_to_string(path).expect("read"), BODY);
    assert!(got.get("content").is_none());
    assert_eq!(status(&get_path(&json!({ "id": "ffffffffffffffffffffffffffffffff" }))), 404);
}

fn write_text(workspace: &str, body: &str) -> String {
    let created = create_doc_content(&json!({
        "workspace": workspace,
        "title": "Searchable",
        "content": body,
        "digest": "never",
    }));
    text(ok(&created), "id")
}

#[test]
fn search_spans_workspaces_or_narrows_to_one() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Memory");
    let in_default = write_text("default", "# Searchable\n\n这是默认空间里的迁移笔记 zebrafish 内容。");
    let in_ws = write_text(&ws, "# Searchable\n\n这是记忆空间里保存的偏好 zebrafish 内容。");

    crate::services::index_build::rebuild_spark_index(&crate::config::paths::repo_root().expect("root")).expect("index");
    let all = search(&json!({ "q": "zebrafish" }));
    let ids: Vec<String> = all["items"].as_array().expect("items").iter().map(|i| text(i, "id")).collect();
    assert!(ids.contains(&in_default) && ids.contains(&in_ws), "{all}");

    let only = search(&json!({ "q": "zebrafish", "workspace": ws }));
    let items = only["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "{only}");
    assert_eq!(text(&items[0], "id"), in_ws);
    assert_eq!(text(&items[0], "workspace"), ws);
    assert_eq!(text(&items[0], "project"), "inbox");
}

#[test]
fn search_checks_its_arguments() {
    let _sandbox = TestSandbox::new();
    assert_eq!(status(&search(&json!({}))), 400);
    assert_eq!(status(&search(&json!({ "q": "x", "workspace": "nope" }))), 404);
}

#[test]
fn a_full_rebuild_indexes_every_workspace_and_drops_chunks_of_deleted_documents() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Memory");
    let kept = write_text(&ws, "# Searchable\n\n这是重建后保留的文档 platypus 内容。");
    let gone = write_text("default", "# Searchable\n\n这是重建后消失的文档 platypus 内容。");
    let root = crate::config::paths::repo_root().expect("root");

    // Remove the row behind the index's back, then rebuild from the tables.
    let conn = spark_data::open().expect("db");
    spark_data::delete_doc(&conn, "doc_default", &gone).expect("delete row");
    crate::services::index_build::rebuild_spark_index(&root).expect("rebuild");

    let found = search(&json!({ "q": "platypus" }));
    let ids: Vec<String> = found["items"].as_array().expect("items").iter().map(|i| text(i, "id")).collect();
    assert_eq!(ids, vec![kept], "{found}");
}
