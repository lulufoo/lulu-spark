//! LW-440: end-to-end smoke through the MCP route table for the unified naming contract.
//! Each step feeds the previous step's `workspace_id` / `project_id` back in unchanged.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::config::paths;
use crate::mcp_host::catalog::{build, group_for_migrated_api};
use crate::services::search_index::rebuild;
use crate::test_support::TestSandbox;

fn call(name: &str, args: Value) -> Value {
    let group = group_for_migrated_api(name).expect(name);
    (build(group, name, "spark").expect(name).invoke)(&args)
}

fn ok(value: Value) -> Value {
    assert_eq!(value.get("ok"), Some(&json!(true)), "expected ok: {value}");
    value
}

fn stage(sandbox: &TestSandbox, name: &str, content: &str) -> String {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let path: PathBuf = dir.join(name);
    fs::write(&path, content).expect("write stage");
    path.canonicalize().expect("canon").to_str().unwrap().to_string()
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("missing `{key}` in {value}"))
        .to_string()
}

fn has_no_old_scope_keys(value: &Value) {
    for key in ["workspace", "project", "topic", "title_of_project"] {
        assert!(value.get(key).is_none(), "old key `{key}` present in {value}");
    }
}

#[test]
fn create_workspace_to_delete_doc_uses_the_unified_keys() {
    let sandbox = TestSandbox::new();
    fs::create_dir_all(sandbox.data_dir().join("notes").join("raw")).expect("raw");
    fs::create_dir_all(sandbox.data_dir().join("notes").join("digest")).expect("digest");

    // An empty index file makes the writes below keep it in step, as the running app does.
    rebuild(&paths::cache_dir().expect("cache"), &[], None, &HashSet::new()).expect("index");

    let ws = ok(call("create_workspace", json!({ "workspace_name": "Smoke WS" })));
    let workspace_id = text(&ws, "workspace_id");
    assert_eq!(text(&ws, "workspace_name"), "Smoke WS");
    has_no_old_scope_keys(&ws);

    let pr = ok(call(
        "create_project",
        json!({ "workspace_id": workspace_id, "project_name": "Smoke Project" }),
    ));
    let project_id = text(&pr, "project_id");
    assert_eq!(text(&pr, "project_name"), "Smoke Project");
    assert!(pr.get("workspace_id").is_none(), "project results do not echo workspace_id: {pr}");
    has_no_old_scope_keys(&pr);

    let listed = call("list_projects", json!({ "workspace_id": workspace_id }));
    let rows = listed["projects"].as_array().expect("projects");
    assert!(rows.iter().any(|row| row["project_id"] == json!(project_id) && row["project_name"] == json!("Smoke Project")));
    for row in rows {
        has_no_old_scope_keys(row);
        assert!(row.get("id").is_none() && row.get("title").is_none(), "old project row keys: {row}");
    }

    // Omitting project_id lands the document in the workspace inbox.
    let created = ok(call(
        "create_doc",
        json!({
            "workspace_id": workspace_id,
            "source_path": stage(&sandbox, "smoke.md", "# Smoke Doc\n\n---\n\nuniquesmokeneedle 正文\n"),
            "title": "Smoke Doc",
            "digest": "never",
            "source_type": "summary",
        }),
    ));
    let doc_id = text(&created, "id");
    assert_eq!(text(&created, "workspace_id"), workspace_id);
    assert_eq!(text(&created, "workspace_name"), "Smoke WS");
    let inbox = rows.iter().find(|row| row["is_inbox"] == json!(true)).expect("inbox row");
    assert_eq!(text(&created, "project_id"), text(inbox, "project_id"));
    assert!(created.get("project_name").is_some(), "{created}");
    has_no_old_scope_keys(&created);

    // Explicit project_id lands there.
    let second = ok(call(
        "create_doc",
        json!({
            "workspace_id": workspace_id,
            "project_id": project_id,
            "source_path": stage(&sandbox, "second.md", "# Second\n\n第二篇正文\n"),
            "title": "Second",
            "digest": "never",
            "source_type": "summary",
        }),
    ));
    assert_eq!(text(&second, "project_id"), project_id);
    assert_eq!(text(&second, "project_name"), "Smoke Project");

    let hits = call("search", json!({ "q": "uniquesmokeneedle", "workspace_id": workspace_id }));
    let list = hits["hits"].as_array().or_else(|| hits["items"].as_array()).unwrap_or_else(|| panic!("hits in {hits}"));
    let hit = list.iter().find(|h| h["id"] == json!(doc_id)).unwrap_or_else(|| panic!("hit in {hits}"));
    for key in ["id", "title", "snippet", "workspace_id", "workspace_name", "project_id", "project_name"] {
        assert!(hit.get(key).is_some(), "search hit lacks `{key}`: {hit}");
    }
    has_no_old_scope_keys(hit);

    let dest = sandbox.cache_dir().join("smoke_dest");
    fs::create_dir_all(&dest).expect("dest");
    let file = ok(call(
        "get_doc_file",
        json!({ "id": doc_id, "workspace_id": workspace_id, "dest_dir": dest.to_str().unwrap() }),
    ));
    let copy = text(&file, "path");
    fs::write(&copy, "# Smoke Doc\n\nuniquesmokeneedle 已编辑\n").expect("edit copy");

    let updated = ok(call(
        "update_doc",
        json!({ "id": doc_id, "workspace_id": workspace_id, "source_path": copy, "digest": "never" }),
    ));
    assert_eq!(text(&updated, "id"), doc_id);
    assert_eq!(text(&updated, "workspace_id"), workspace_id);
    assert_eq!(text(&updated, "project_id"), text(inbox, "project_id"));
    has_no_old_scope_keys(&updated);

    let removed = ok(call("delete_doc", json!({ "id": doc_id, "workspace_id": workspace_id })));
    assert_eq!(text(&removed, "id"), doc_id);
    assert!(removed.get("removed").is_some(), "{removed}");
}

/// Hard cut: the retired `workspace` / `project` keys are ignored, so the write lands in
/// `Default` / inbox. The Host does not reject them; only a client that validates the
/// `additionalProperties: false` schema would.
#[test]
fn retired_scope_keys_are_ignored_and_land_in_default() {
    let sandbox = TestSandbox::new();
    fs::create_dir_all(sandbox.data_dir().join("notes").join("raw")).expect("raw");
    fs::create_dir_all(sandbox.data_dir().join("notes").join("digest")).expect("digest");

    let ws = ok(call("create_workspace", json!({ "workspace_name": "Elsewhere" })));
    let elsewhere = text(&ws, "workspace_id");

    let created = ok(call(
        "create_doc",
        json!({
            "workspace": elsewhere,
            "project": "does-not-exist",
            "source_path": stage(&sandbox, "old.md", "# Old keys\n\n旧键正文\n"),
            "title": "Old keys",
            "digest": "never",
            "source_type": "summary",
        }),
    ));
    assert_eq!(text(&created, "workspace_id"), crate::test_support::default_workspace().id);
    assert_ne!(text(&created, "workspace_id"), elsewhere);

    let renamed = call("rename_workspace", json!({ "workspace": elsewhere, "workspace_name": "X" }));
    assert!(renamed.get("error").is_some(), "old workspace key must not address the workspace: {renamed}");
}
