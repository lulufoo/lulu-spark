use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use super::*;
use crate::services::spark_data::{self, get_workspace};
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
    let made = create_workspace(&json!({ "workspace_name": name }));
    text(&ok(&made), "workspace_id")
}

fn inbox_of(workspace: &str) -> String {
    spark_data::get_inbox_project(&spark_data::open().expect("db"), workspace)
        .expect("lookup")
        .expect("inbox")
        .id
}

fn new_project(workspace: Option<&str>, title: &str) -> String {
    let mut payload = json!({ "project_name": title });
    if let Some(ws) = workspace {
        payload["workspace_id"] = json!(ws);
    }
    text(ok(&create_project(&payload)), "project_id")
}

fn project_ids(value: &Value) -> Vec<String> {
    value["projects"]
        .as_array()
        .expect("array")
        .iter()
        .map(|p| text(p, "project_id"))
        .collect()
}

fn write(workspace: Option<&str>, project: Option<&str>) -> Value {
    let mut payload = json!({ "title": "T", "content": BODY, "digest": "never", "source_type": "summary" });
    if let Some(w) = workspace {
        payload["workspace_id"] = json!(w);
    }
    if let Some(p) = project {
        payload["project_id"] = json!(p);
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
    let inbox = inbox_of(&ws);
    assert_eq!((text(&created, "workspace_id"), text(&created, "project_id")), (ws.clone(), inbox.clone()));

    let on_disk = fs::read_dir(workspaces_dir(&sandbox).join(&ws).join("raw").join(&inbox).join("notes")).expect("dir");
    assert_eq!(on_disk.count(), 1);
    let read = get_file(&json!({ "workspace_id": ws, "id": id }));
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
    assert_eq!((text(&created, "workspace_id"), text(&created, "project_id")), ("default".to_string(), "inbox".to_string()));
    assert!(workspaces_dir(&sandbox).join("default/raw/inbox/notes").is_dir());
    assert_eq!(get_file(&json!({ "id": id }))["content"], json!(BODY));
}

#[test]
fn a_project_of_another_workspace_is_rejected() {
    let _sandbox = TestSandbox::new();
    let alpha = new_project(None, "Alpha");
    let other = new_workspace("Other");

    assert_eq!(status(&write(Some(&other), Some(&alpha))), 400, "alpha is a default project");
    ok(&write(None, Some(&alpha)));
    assert_eq!(status(&write(None, Some("nope"))), 400);
    assert_eq!(status(&write(Some("missing"), None)), 404);
}

#[test]
fn a_document_is_only_found_in_its_own_workspace() {
    let _sandbox = TestSandbox::new();
    let other = new_workspace("Other");
    let id = text(ok(&write(None, None)), "id");

    assert_eq!(status(&get_file(&json!({ "workspace_id": other, "id": id }))), 404);
    let update = update_doc_content(&json!({ "workspace_id": other, "id": id, "content": BODY, "digest": "never" }));
    assert_eq!(status(&update), 404);
    assert_eq!(status(&delete_doc(&json!({ "workspace_id": other, "id": id }))), 404);
    ok(&delete_doc(&json!({ "id": id })));
}

#[test]
fn update_overwrites_the_body_and_refuses_create_only_fields() {
    let _sandbox = TestSandbox::new();
    let id = text(ok(&write(None, None)), "id");
    let newer = "# 新标题\n\n更新后的正文。\n";
    ok(&update_doc_content(&json!({ "id": id, "content": newer, "digest": "never" })));
    assert_eq!(get_file(&json!({ "id": id }))["content"], json!(newer));

    for key in ["title", "project_id", "folder", "theme", "created_at", "source_type"] {
        let mut payload = json!({ "id": id, "content": newer, "digest": "never" });
        payload[key] = json!("x");
        assert_eq!(status(&update_doc_content(&payload)), 400, "{key}");
    }
}

#[test]
fn a_digest_is_written_and_read_back() {
    let _sandbox = TestSandbox::new();
    let created = create_doc_content(&json!({
        "title": "T", "content": BODY, "digest": "always", "digest_body": "# 摘要\n\n要点。\n", "source_type": "summary"
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
        "title": "T", "content": BODY, "digest": "always", "digest_body": "# 摘要\n\n要点。\n", "source_type": "summary"
    }));
    let id = text(ok(&created), "id");
    let removed = delete_doc(&json!({ "id": id }));
    assert_eq!(ok(&removed)["removed"], json!(2));
    assert_eq!(status(&get_file(&json!({ "id": id }))), 404);

    let leftover = fs::read_dir(workspaces_dir(&sandbox).join("default/raw/inbox/notes")).expect("dir").count();
    assert_eq!(leftover, 0);
}

#[test]
fn default_workspace_cannot_be_disabled_and_the_default_can_be_renamed() {
    let _sandbox = TestSandbox::new();
    assert_eq!(status(&disable_workspace(&json!({ "workspace_id": "default" }))), 400);
    let renamed = rename_workspace(&json!({ "workspace_id": "default", "workspace_name": "Home" }));
    assert_eq!(ok(&renamed)["workspace_name"], json!("Home"));
    assert_eq!(ok(&renamed)["workspace_id"], json!("default"));
    let listed = list_workspaces();
    assert_eq!(listed["workspaces"][0]["workspace_id"], json!("default"), "default comes first");
}

#[test]
fn a_disabled_workspace_is_hidden_keeps_its_documents_and_can_be_enabled_again() {
    let sandbox = TestSandbox::new();
    let ws = new_workspace("Temp");
    let id = text(ok(&write(Some(&ws), None)), "id");

    let disabled = disable_workspace(&json!({ "workspace_id": ws }));
    assert!(ok(&disabled)["disabled_at"].is_string());
    let names = |v: &Value| -> Vec<String> {
        v["workspaces"].as_array().expect("array").iter().map(|w| text(w, "workspace_id")).collect()
    };
    assert!(!names(&list_workspaces()).contains(&ws), "hidden from the list");
    assert_eq!(names(&list_disabled_workspaces()), vec![ws.clone()]);

    // The data stays: files, rows, and the document still opens by its id.
    assert!(workspaces_dir(&sandbox).join(&ws).exists());
    assert_eq!(get_file(&json!({ "id": id }))["content"], json!(BODY));
    let conn = spark_data::open().expect("db");
    let row = get_workspace(&conn, &ws).expect("get").expect("row");
    assert!(spark_data::list_doc_entries(&conn, &row).expect("entries").is_empty(), "hidden from the index");

    // Writes are refused and the name stays taken.
    assert_eq!(status(&write(Some(&ws), None)), 409);
    let again = create_workspace(&json!({ "workspace_name": "Temp" }));
    assert_eq!(status(&again), 409);
    assert!(text(&again, "error").contains("disabled"), "{again}");

    ok(&enable_workspace(&json!({ "workspace_id": ws })));
    assert!(names(&list_workspaces()).contains(&ws));
    assert!(names(&list_disabled_workspaces()).is_empty());
    let row = get_workspace(&conn, &ws).expect("get").expect("row");
    assert_eq!(spark_data::list_doc_entries(&conn, &row).expect("entries").len(), 1);
    ok(&write(Some(&ws), None));
    assert_eq!(status(&disable_workspace(&json!({ "workspace_id": "ghost" }))), 404);
}

#[test]
fn workspace_names_and_view_modes_are_checked() {
    let _sandbox = TestSandbox::new();
    let made = create_workspace(&json!({ "workspace_name": "Notes", "view_mode": "timeline" }));
    assert_eq!(ok(&made)["view_mode"], json!("timeline"));
    assert_eq!(ok(&made)["is_default"], json!(false));
    assert_eq!(status(&create_workspace(&json!({ "workspace_name": "Notes" }))), 409, "name is unique");
    assert_eq!(status(&create_workspace(&json!({ "workspace_name": "  " }))), 400);
    assert_eq!(status(&create_workspace(&json!({ "workspace_name": "X", "view_mode": "grid" }))), 400);

    let ws = text(&made, "workspace_id");
    let switched = set_view_mode(&json!({ "workspace_id": ws, "view_mode": "directory" }));
    assert_eq!(ok(&switched)["view_mode"], json!("directory"));
    assert_eq!(status(&set_view_mode(&json!({ "workspace_id": ws }))), 400);
}

#[test]
fn project_crud_follows_the_workspace_and_protects_inbox_and_documents() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Work");
    let made = create_project(&json!({ "workspace_id": ws, "project_name": "Plans", "description": "d" }));
    let plans = text(&ok(&made), "project_id");
    assert!(ok(&made).get("folder").is_none(), "{made}");
    assert_eq!(status(&create_project(&json!({ "workspace_id": ws, "project_name": " " }))), 400);

    let minted = create_project(&json!({ "workspace_id": ws, "project_name": "Auto" }));
    assert!(ok(&minted).get("folder").is_none(), "{minted}");

    let listed = list_projects(&json!({ "workspace_id": ws }));
    let inbox = listed["projects"]
        .as_array()
        .expect("array")
        .iter()
        .find(|p| p["is_inbox"] == true)
        .expect("created workspace has an inbox");
    let inbox_id = text(inbox, "project_id");
    assert_eq!(text(inbox, "project_name"), "Inbox");
    assert!(inbox.get("folder").is_none(), "{inbox}");
    assert_ne!(inbox_id, "inbox");
    assert!(project_ids(&listed).contains(&plans), "{listed}");

    let updated = update_project(&json!({ "workspace_id": ws, "project_id": plans, "project_name": "Roadmaps" }));
    assert_eq!(
        (text(&ok(&updated), "project_name"), text(&updated, "project_id")),
        ("Roadmaps".into(), plans.clone())
    );
    assert_eq!(status(&update_project(&json!({ "workspace_id": ws, "project_id": "ghost", "project_name": "x" }))), 400);
    assert_eq!(
        status(&update_project(&json!({ "workspace_id": ws, "project_id": "inbox", "project_name": "Catch-all" }))),
        400,
        "old folder string is not an id"
    );

    ok(&update_project(&json!({ "workspace_id": ws, "project_id": inbox_id, "project_name": "Catch-all" })));
}

#[test]
fn a_project_row_with_a_legacy_disabled_at_is_an_ordinary_project() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Work");
    let plans = new_project(Some(&ws), "Plans");
    let id = text(ok(&write(Some(&ws), Some(&plans))), "id");

    // Older versions could disable a project; that column is no longer read.
    let conn = spark_data::open().expect("db");
    conn.execute("UPDATE project SET disabled_at = '2026-01-01T00:00:00Z' WHERE id = ?1", [&plans])
        .expect("legacy mark");

    assert!(project_ids(&list_projects(&json!({ "workspace_id": ws }))).contains(&plans));
    let row = get_workspace(&conn, &ws).expect("get").expect("row");
    assert!(spark_data::list_doc_entries(&conn, &row).expect("entries").contains_key(&id));
    ok(&write(Some(&ws), Some(&plans)));
    let update = update_doc_content(&json!({ "workspace_id": ws, "id": id, "content": BODY, "digest": "never" }));
    ok(&update);
    let other = new_project(Some(&ws), "Other");
    ok(&move_doc(&json!({ "workspace_id": ws, "id": id, "project_id": other })));
    ok(&update_project(&json!({ "workspace_id": ws, "project_id": plans, "project_name": "Roadmaps" })));
}

#[test]
fn a_disabled_workspace_refuses_new_projects_until_it_is_enabled() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Work");
    ok(&disable_workspace(&json!({ "workspace_id": ws })));
    assert_eq!(status(&create_project(&json!({ "workspace_id": ws, "project_name": "X" }))), 409);
    ok(&enable_workspace(&json!({ "workspace_id": ws })));
    ok(&create_project(&json!({ "workspace_id": ws, "project_name": "X" })));
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
        "workspace_id": workspace,
        "title": "Searchable",
        "content": body,
        "digest": "never",
        "source_type": "summary",
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

    let only = search(&json!({ "q": "zebrafish", "workspace_id": ws }));
    let items = only["items"].as_array().expect("items");
    assert_eq!(items.len(), 1, "{only}");
    assert_eq!(text(&items[0], "id"), in_ws);
    assert_eq!(text(&items[0], "workspace_id"), ws);
    assert_eq!(text(&items[0], "project_id"), inbox_of(&ws));
    assert_eq!(text(&items[0], "workspace_name"), "Memory");
    assert_eq!(text(&items[0], "project_name"), "Inbox");
    assert!(items[0].get("workspace").is_none() && items[0].get("project").is_none(), "{only}");
}

#[test]
fn search_checks_its_arguments() {
    let _sandbox = TestSandbox::new();
    assert_eq!(status(&search(&json!({}))), 400);
    assert_eq!(status(&search(&json!({ "q": "x", "workspace_id": "nope" }))), 404);
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

fn write_in_folder(folder: &str, extra: Value) -> Value {
    let mut payload = json!({ "title": "Nested", "content": BODY, "digest": "never", "source_type": "summary", "folder": folder });
    for (key, value) in extra.as_object().expect("object") {
        payload[key] = value.clone();
    }
    create_doc_content(&payload)
}

#[test]
fn a_three_level_folder_works_through_create_read_move_search_and_delete() {
    let sandbox = TestSandbox::new();
    let plans = new_project(None, "Plans");
    let created = write_in_folder("ai/agents/notes", json!({ "translations": [{ "lang": "fr", "content": "# Titre\n\ncorps\n" }] }));
    let id = text(ok(&created), "id");

    let rel = spark_data::find_doc(&spark_data::open().expect("db"), &id).expect("find").expect("doc").doc.raw_path;
    assert!(rel.starts_with("ai/agents/notes/"), "{rel}");
    let dir = workspaces_dir(&sandbox).join("default/raw/inbox/ai/agents/notes");
    assert_eq!(fs::read_dir(&dir).expect("dir").count(), 2, "the document and its translation");
    assert_eq!(get_file(&json!({ "id": id }))["content"], json!(BODY));
    assert_eq!(get_file(&json!({ "id": id, "lang": "fr" }))["content"], json!("# Titre\n\ncorps"));

    let updated = update_doc_content(&json!({ "id": id, "content": "# 新\n\n更新后的正文 narwhal。\n", "digest": "never" }));
    ok(&updated);
    crate::services::index_build::rebuild_spark_index(&crate::config::paths::repo_root().expect("root")).expect("index");
    let found = search(&json!({ "q": "narwhal" }));
    assert_eq!(text(&found["items"][0], "id"), id, "{found}");

    assert_eq!(status(&move_doc(&json!({ "id": id, "project": plans }))), 400, "the old key is not read");
    let moved = move_doc(&json!({ "id": id, "project_id": plans }));
    assert_eq!(
        [text(ok(&moved), "workspace_id"), text(&moved, "workspace_name"), text(&moved, "project_id"), text(&moved, "project_name")],
        ["default".to_string(), "Default".to_string(), plans.clone(), "Plans".to_string()],
        "{moved}"
    );
    assert!(workspaces_dir(&sandbox).join("default/raw").join(&plans).join("ai/agents/notes").is_dir());
    assert_eq!(fs::read_dir(&dir).expect("old dir").count(), 0, "both files left the old project");
    assert_eq!(get_file(&json!({ "id": id, "lang": "fr" }))["content"], json!("# Titre\n\ncorps"));

    assert_eq!(ok(&delete_doc(&json!({ "id": id })))["removed"], json!(2));
    assert_eq!(status(&get_file(&json!({ "id": id }))), 404);
}

#[test]
fn a_folder_with_four_levels_or_unsafe_segments_is_rejected() {
    let sandbox = TestSandbox::new();
    for bad in ["a/b/c/d", "a//b", "/a", "a/", "../x", "a/../b", "a/./b", "a\\b", "a..b"] {
        let v = write_in_folder(bad, json!({}));
        assert_eq!(status(&v), 400, "folder {bad:?} gave {v}");
    }
    assert!(!workspaces_dir(&sandbox).join("default/raw").join("inbox").exists(), "nothing is written for a rejected folder");
    for good in ["notes", "a/b", "a/b/c"] {
        ok(&write_in_folder(good, json!({})));
    }
}

fn plain_hex_row(id: &str, project_id: &str) -> spark_data::DocRow {
    let mut row = spark_data::DocRow::new(id, "notes/202601011200-old123.md", project_id);
    row.title = Some("Old".into());
    row
}

fn inbox_id(workspace: &str) -> String {
    inbox_of(workspace)
}

#[test]
fn a_new_doc_id_starts_with_its_workspace_id_and_survives_a_rename() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Memory");
    let in_default = text(ok(&write(None, None)), "id");
    let in_ws = text(ok(&write(Some(&ws), None)), "id");

    let (prefix, hex) = in_default.split_once('-').expect("prefixed");
    assert_eq!(prefix, "default");
    assert_eq!(hex.len(), 32);
    assert!(in_ws.starts_with(&format!("{ws}-")), "{in_ws}");

    ok(&rename_workspace(&json!({ "workspace_id": ws, "workspace_name": "Renamed" })));
    assert_eq!(get_file(&json!({ "id": in_ws }))["content"], json!(BODY), "found by the id alone, no workspace given");
    assert_eq!(text(&get_path(&json!({ "id": in_ws })), "id"), in_ws);
    ok(&delete_doc(&json!({ "id": in_ws })));
    assert_eq!(status(&get_file(&json!({ "id": in_ws }))), 404);
}

#[test]
fn a_prefixed_id_only_looks_in_the_workspace_it_names() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Memory");
    let id = text(ok(&write(Some(&ws), None)), "id");
    let hex = id.split_once('-').expect("prefixed").1;

    assert_eq!(status(&get_file(&json!({ "id": format!("default-{hex}") }))), 404);
    assert_eq!(status(&get_file(&json!({ "id": format!("nope-{hex}") }))), 404);
    assert_eq!(status(&get_file(&json!({ "id": format!("Bad-Prefix-{hex}") }))), 400);
    assert_eq!(status(&get_file(&json!({ "id": format!("{ws}-short") }))), 400);
}

#[test]
fn a_plain_32_hex_id_from_before_the_prefix_still_reads_updates_and_deletes() {
    let sandbox = TestSandbox::new();
    ok(&write(None, None));
    let old = "0123456789abcdef0123456789abcdef";
    let conn = spark_data::open().expect("db");
    spark_data::insert_doc(&conn, "doc_default", &plain_hex_row(old, &inbox_id("default"))).expect("insert");
    let dir = workspaces_dir(&sandbox).join("default/raw/inbox/notes");
    fs::write(dir.join("202601011200-old123.md"), "# Old\n\n旧文档正文。\n").expect("file");

    assert_eq!(get_file(&json!({ "id": old }))["content"], json!("# Old\n\n旧文档正文。\n"));
    ok(&update_doc_content(&json!({ "id": old, "content": "# Old\n\n改过。\n", "digest": "never" })));
    assert_eq!(get_file(&json!({ "id": old }))["content"], json!("# Old\n\n改过。\n"));
    ok(&delete_doc(&json!({ "id": old })));
    assert_eq!(status(&get_file(&json!({ "id": old }))), 404);
}

#[test]
fn an_id_shared_by_two_workspace_tables_is_reported() {
    let _sandbox = TestSandbox::new();
    ok(&write(None, None));
    let conn = spark_data::open().expect("db");
    assert!(spark_data::duplicate_doc_ids(&conn).expect("scan").is_empty());

    let shared = "fedcba9876543210fedcba9876543210";
    let project = inbox_id("default");
    for table in ["doc_default", "doc_knowledge"] {
        spark_data::insert_doc(&conn, table, &plain_hex_row(shared, &project)).expect("insert");
    }
    let dups = spark_data::duplicate_doc_ids(&conn).expect("scan");
    assert_eq!(dups, vec![(shared.to_string(), vec!["default".to_string(), "knowledge".to_string()])]);
}

#[test]
fn search_and_create_return_the_workspace_name_a_chat_link_shows() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Memory");
    let created = write(Some(&ws), None);
    assert_eq!(text(ok(&created), "workspace_name"), "Memory");
    let id = text(&created, "id");

    ok(&rename_workspace(&json!({ "workspace_id": ws, "workspace_name": "Research" })));
    crate::services::index_build::rebuild_spark_index(&crate::config::paths::repo_root().expect("root")).expect("index");
    let found = search(&json!({ "q": "这是正文" }));
    let hit = found["items"].as_array().expect("items").iter().find(|i| text(i, "id") == id).expect("hit");
    assert_eq!(text(hit, "workspace_name"), "Research", "the current name, after the rename");
    assert_eq!(text(hit, "workspace_id"), ws);
}

fn icon_of(value: &Value) -> String {
    text(value, "icon")
}

#[test]
fn a_workspace_icon_is_set_on_create_changed_cleared_and_checked() {
    let _sandbox = TestSandbox::new();
    let plain = create_workspace(&json!({ "workspace_name": "Plain" }));
    assert_eq!(icon_of(&ok(&plain)), "", "no icon means empty");

    let made = create_workspace(&json!({ "workspace_name": "Iconic", "icon": "03ab12" }));
    assert_eq!(icon_of(&ok(&made)), "03ab12");
    let ws = text(&made, "workspace_id");

    let listed = list_workspaces();
    let row = listed["workspaces"].as_array().expect("array").iter().find(|w| text(w, "workspace_id") == ws).expect("row").clone();
    assert_eq!(icon_of(&row), "03ab12");

    assert_eq!(icon_of(&ok(&set_icon(&json!({ "workspace_id": ws, "icon": "05zz99" })))), "05zz99");
    assert_eq!(icon_of(&ok(&set_icon(&json!({ "workspace_id": ws, "icon": "" })))), "", "empty clears");
    assert_eq!(icon_of(&ok(&set_icon(&json!({ "workspace_id": ws, "icon": "  04a0b0 " })))), "04a0b0", "trimmed");

    // A well-formed id the frontend does not know is kept; the backend cannot tell.
    assert_eq!(icon_of(&ok(&set_icon(&json!({ "workspace_id": ws, "icon": "00zzzz" })))), "00zzzz");

    for bad in ["7", "07abcd", "03ABCD", "03abc", "03abcde", "03ab-d", "folder"] {
        assert_eq!(status(&set_icon(&json!({ "workspace_id": ws, "icon": bad }))), 400, "{bad}");
    }
    assert_eq!(status(&set_icon(&json!({ "workspace_id": ws, "icon": 5 }))), 400);
    assert_eq!(status(&set_icon(&json!({ "workspace_id": ws }))), 400, "icon is required here");
    assert_eq!(status(&set_icon(&json!({ "icon": "03ab12" }))), 400);
    assert_eq!(status(&set_icon(&json!({ "workspace_id": "ghost", "icon": "03ab12" }))), 404);

    // Rejected input never leaves a half-made workspace behind.
    assert_eq!(status(&create_workspace(&json!({ "workspace_name": "Bad", "icon": "nope" }))), 400);
    let names: Vec<_> = list_workspaces()["workspaces"].as_array().expect("array").iter().map(|w| text(w, "workspace_name")).collect();
    assert!(!names.contains(&"Bad".to_string()), "{names:?}");
}

#[test]
fn a_disabled_workspace_keeps_and_lists_its_icon() {
    let _sandbox = TestSandbox::new();
    let ws = text(ok(&create_workspace(&json!({ "workspace_name": "Temp", "icon": "02aa11" }))), "workspace_id");
    ok(&disable_workspace(&json!({ "workspace_id": ws })));
    let disabled = list_disabled_workspaces();
    assert_eq!(icon_of(&disabled["workspaces"][0]), "02aa11");
    assert_eq!(icon_of(&ok(&enable_workspace(&json!({ "workspace_id": ws })))), "02aa11");
}

#[test]
fn a_project_icon_is_kept_unless_changed_and_cleared_by_an_empty_string() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Work");
    let plain = create_project(&json!({ "workspace_id": ws, "project_name": "Plain" }));
    assert_eq!(icon_of(&ok(&plain)), "");

    let made = create_project(&json!({ "workspace_id": ws, "project_name": "Plans", "icon": "03k7x2" }));
    let plans = text(&ok(&made), "project_id");
    assert_eq!(icon_of(&ok(&made)), "03k7x2");

    let renamed = update_project(&json!({ "workspace_id": ws, "project_id": plans, "project_name": "Roadmaps" }));
    assert_eq!(icon_of(&ok(&renamed)), "03k7x2", "absent icon keeps it");
    let changed = update_project(&json!({ "workspace_id": ws, "project_id": plans, "project_name": "Roadmaps", "icon": "01aa00" }));
    assert_eq!(icon_of(&ok(&changed)), "01aa00");
    let cleared = update_project(&json!({ "workspace_id": ws, "project_id": plans, "project_name": "Roadmaps", "icon": "" }));
    assert_eq!(icon_of(&ok(&cleared)), "");

    let before = project_ids(&list_projects(&json!({ "workspace_id": ws })));
    assert_eq!(status(&create_project(&json!({ "workspace_id": ws, "project_name": "X", "icon": "9" }))), 400);
    assert_eq!(status(&update_project(&json!({ "workspace_id": ws, "project_id": plans, "project_name": "R", "icon": "bad" }))), 400);
    assert_eq!(
        project_ids(&list_projects(&json!({ "workspace_id": ws }))),
        before,
        "a rejected create leaves nothing"
    );

    ok(&update_project(&json!({ "workspace_id": ws, "project_id": plans, "project_name": "R", "icon": "02bb22" })));
    let listed = list_projects(&json!({ "workspace_id": ws }));
    let row = listed["projects"].as_array().expect("array").iter().find(|p| text(p, "project_id") == plans).expect("row").clone();
    assert_eq!(icon_of(&row), "02bb22");
}

#[test]
fn workspaces_and_projects_can_be_reordered_through_the_service() {
    let _sandbox = TestSandbox::new();
    let a = new_workspace("Aaa");
    let b = new_workspace("Bbb");
    let ids = |v: &Value| -> Vec<String> {
        v["workspaces"].as_array().expect("array").iter().map(|w| text(w, "workspace_id")).collect()
    };
    assert_eq!(ids(&list_workspaces()), vec!["default".to_string(), "knowledge".into(), a.clone(), b.clone()]);

    let moved = reorder_workspaces(&json!({ "ids": [b, "default", a, "knowledge"] }));
    assert_eq!(ids(ok(&moved)), vec![b.clone(), "default".to_string(), a.clone(), "knowledge".into()]);
    assert_eq!(ids(&list_workspaces()), ids(&moved), "the list follows the new order");
    let indexes: Vec<i64> = list_workspaces()["workspaces"]
        .as_array()
        .expect("array")
        .iter()
        .map(|w| w["sort_index"].as_i64().expect("sort_index"))
        .collect();
    assert_eq!(indexes, vec![0, 1, 2, 3], "rows expose their position for the Settings drag lists");

    assert_eq!(status(&reorder_workspaces(&json!({}))), 400, "ids is required");
    assert_eq!(status(&reorder_workspaces(&json!({ "ids": "default" }))), 400);
    assert_eq!(status(&reorder_workspaces(&json!({ "ids": [1] }))), 400);
    assert_eq!(status(&reorder_workspaces(&json!({ "ids": ["default"] }))), 400, "must name every workspace");
    assert_eq!(ids(&list_workspaces()), ids(&moved), "a refused reorder changes nothing");

    let inbox = inbox_of(&a);
    let one = new_project(Some(&a), "one");
    let two = new_project(Some(&a), "two");
    let three = new_project(Some(&a), "three");
    assert_eq!(project_ids(&list_projects(&json!({ "workspace_id": a }))), vec![inbox.clone(), one.clone(), two.clone(), three.clone()]);
    let reordered = reorder_projects(&json!({ "workspace_id": a, "ids": [three.clone(), inbox.clone(), two.clone(), one.clone()] }));
    assert_eq!(project_ids(ok(&reordered)), vec![three.clone(), inbox.clone(), two.clone(), one.clone()]);
    assert_eq!(project_ids(&list_projects(&json!({ "workspace_id": a }))), project_ids(&reordered));

    assert_eq!(status(&reorder_projects(&json!({ "workspace_id": a }))), 400);
    assert_eq!(status(&reorder_projects(&json!({ "workspace_id": a, "ids": [one] }))), 400);
    assert_eq!(status(&reorder_projects(&json!({ "workspace_id": "ghost", "ids": [] }))), 404);
    ok(&disable_workspace(&json!({ "workspace_id": b })));
    assert!(status(&reorder_projects(&json!({ "workspace_id": b, "ids": [inbox_of(&b)] }))) >= 400, "disabled workspace");
}

#[test]
fn workspace_and_project_results_are_flat_and_keyed_by_resource() {
    let _sandbox = TestSandbox::new();
    let made = ok(&create_workspace(&json!({ "workspace_name": "Flat" }))).clone();
    for key in ["ok", "workspace_id", "workspace_name", "view_mode", "is_default", "created_at", "disabled_at", "icon", "sort_index"] {
        assert!(made.get(key).is_some(), "workspace result lacks {key}: {made}");
    }
    for key in ["workspace", "id", "name"] {
        assert!(made.get(key).is_none(), "workspace result keeps the old key {key}: {made}");
    }
    let ws = text(&made, "workspace_id");

    let project = create_project(&json!({ "workspace_id": ws, "project_name": "Plans" }));
    let mut keys: Vec<&str> = ok(&project).as_object().expect("object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["description", "icon", "is_inbox", "ok", "project_id", "project_name", "sort_index"],
        "a project result is flat and does not echo workspace_id: {project}"
    );
    let listed = list_projects(&json!({ "workspace_id": ws }));
    for item in listed["projects"].as_array().expect("array") {
        assert!(item.get("workspace_id").is_none(), "{item}");
        assert!(item.get("id").is_none() && item.get("title").is_none(), "{item}");
    }
}

#[test]
fn the_old_workspace_and_project_keys_are_ignored() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Work");

    assert_eq!(status(&create_workspace(&json!({ "name": "Old" }))), 400, "name no longer names a workspace");
    assert_eq!(status(&rename_workspace(&json!({ "id": ws, "name": "Old" }))), 400);
    assert_eq!(status(&set_view_mode(&json!({ "id": ws, "view_mode": "timeline" }))), 400);
    assert_eq!(status(&disable_workspace(&json!({ "id": ws }))), 400);
    assert_eq!(status(&create_project(&json!({ "title": "Old" }))), 400, "title no longer names a project");

    let plans = new_project(Some(&ws), "Plans");
    assert_eq!(status(&update_project(&json!({ "workspace_id": ws, "project": plans, "title": "x" }))), 400);

    // `workspace` is not read any more: the project lands in Default, not in `ws`.
    let stray = create_project(&json!({ "workspace": ws, "project_name": "Stray" }));
    let stray_id = text(ok(&stray), "project_id");
    assert!(project_ids(&list_projects(&json!({}))).contains(&stray_id));
    assert!(!project_ids(&list_projects(&json!({ "workspace_id": ws }))).contains(&stray_id));
}

#[test]
fn write_results_carry_the_four_scope_fields_and_delete_carries_none() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Scoped");
    let plans = new_project(Some(&ws), "Plans");

    let created = write(Some(&ws), Some(&plans));
    let id = text(ok(&created), "id");
    assert_eq!(
        [text(&created, "workspace_id"), text(&created, "workspace_name"), text(&created, "project_id"), text(&created, "project_name")],
        [ws.clone(), "Scoped".to_string(), plans.clone(), "Plans".to_string()],
    );
    for key in ["workspace", "project"] {
        assert!(created.get(key).is_none(), "create_doc keeps the old key {key}: {created}");
    }

    let inbox = write(Some(&ws), None);
    assert_eq!(text(&inbox, "project_id"), inbox_of(&ws));
    assert_eq!(text(&inbox, "project_name"), "Inbox", "omitting project_id names the workspace inbox");

    let updated = update_doc_content(&json!({ "workspace_id": ws, "id": id, "content": BODY, "digest": "never" }));
    assert_eq!(
        [text(ok(&updated), "workspace_id"), text(&updated, "project_id"), text(&updated, "project_name")],
        [ws.clone(), plans.clone(), "Plans".to_string()],
        "update_doc names the project the document lives in"
    );

    let deleted = delete_doc(&json!({ "workspace_id": ws, "id": id }));
    assert_eq!(ok(&deleted), &json!({ "ok": true, "id": id, "removed": 1 }));
}

#[test]
fn the_old_workspace_key_is_ignored_by_the_document_tools() {
    let _sandbox = TestSandbox::new();
    let ws = new_workspace("Other");
    let id = text(ok(&write(Some(&ws), None)), "id");

    // `workspace` is not read: create lands in Default, not in `ws`.
    let stray = create_doc_content(&json!({ "workspace": ws, "title": "T", "content": BODY, "digest": "never", "source_type": "summary" }));
    assert_eq!(text(ok(&stray), "workspace_id"), "default");
    // `project` is not read either: the document goes to the inbox.
    let plans = new_project(Some(&ws), "Plans");
    let inboxed = create_doc_content(&json!({ "workspace_id": ws, "project": plans, "title": "T", "content": BODY, "digest": "never", "source_type": "summary" }));
    assert_eq!(text(ok(&inboxed), "project_id"), inbox_of(&ws));
    // A prefixed id still names its workspace, so the old key changes nothing on reads.
    assert_eq!(get_file(&json!({ "workspace": "default", "id": id }))["content"], json!(BODY));
}
