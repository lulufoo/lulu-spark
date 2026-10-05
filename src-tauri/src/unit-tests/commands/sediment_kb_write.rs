use serde_json::json;

use crate::commands::write::{
    map_sediment_kb_error, sediment_kb_add_category_json, sediment_kb_add_repo_json,
    sediment_kb_remove_category_json, sediment_kb_remove_repo_json, sediment_kb_rename_category_json,
    sediment_kb_update_repo_category_json,
};
use crate::services::sediment_kb::{
    add_directory, add_category, ensure_uncategorized, load_repos, SedimentKbError,
    UNCATEGORIZED_ID, update_repo_category,
};
use crate::test_support::TestSandbox;

fn with_sediment_kb_cache<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    crate::config::secrets::test_secrets_clear();
    f();
}

#[test]
fn sediment_kb_add_repo_json_adds_directory_record() {
    with_sediment_kb_cache(|| {
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_repo_json(json!({ "name": "demo" })).expect("add");
        assert!(v.get("error").is_none());
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].full_name, "demo");
        assert_eq!(repos.repos[0].category_id, UNCATEGORIZED_ID);
        assert_eq!(repos.repos[0].description, "");
        let dir = crate::config::paths::knowledge_root()
            .expect("kb")
            .join("demo");
        assert!(dir.is_dir());
    });
}

#[test]
fn sediment_kb_add_repo_json_requires_name() {
    let err = sediment_kb_add_repo_json(json!({ "full_name": "acme/demo" }))
        .expect_err("missing name should be an Err");
    assert_eq!(err, "missing name");
}

#[test]
fn sediment_kb_add_repo_json_rejects_path_name() {
    with_sediment_kb_cache(|| {
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_repo_json(json!({ "name": "acme/demo" })).expect("err");
        assert_eq!(v["code"], "invalid_format");
    });
}

#[test]
fn sediment_kb_add_repo_json_crud_roundtrip() {
    with_sediment_kb_cache(|| {
        ensure_uncategorized().expect("ensure");
        sediment_kb_add_repo_json(json!({ "name": "a" })).expect("add");
        let cat_id = add_category("Docs").expect("cat");
        sediment_kb_update_repo_category_json(json!({
            "full_name": "a",
            "category_id": cat_id
        }))
        .expect("update");
        sediment_kb_remove_repo_json(json!({ "full_name": "a" })).expect("remove");
        assert!(load_repos().expect("load").repos.is_empty());
        assert!(crate::config::paths::knowledge_root()
            .expect("kb")
            .join("a")
            .is_dir());
    });
}

#[test]
fn sediment_kb_category_crud() {
    with_sediment_kb_cache(|| {
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_category_json(json!({ "name": "AI" })).expect("add cat");
        let id = v["id"].as_str().expect("id");
        sediment_kb_rename_category_json(json!({ "id": id, "name": "ML" })).expect("rename");
        add_directory("x").expect("seed");
        update_repo_category("x", id).expect("cat");
        sediment_kb_remove_category_json(json!({ "id": id })).expect("remove cat");
        let repos = load_repos().expect("repos");
        assert!(repos.repos.iter().all(|r| r.category_id == UNCATEGORIZED_ID));
    });
}

#[test]
fn map_sediment_kb_error_codes() {
    let invalid = map_sediment_kb_error(SedimentKbError::invalid_format());
    assert_eq!(invalid["code"], "invalid_format");
    assert_eq!(invalid["_status"], 400);

    let dup = map_sediment_kb_error(SedimentKbError::Duplicate);
    assert_eq!(dup["code"], "duplicate");

    let na = map_sediment_kb_error(SedimentKbError::not_accessible("repo not found"));
    assert_eq!(na["code"], "not_accessible");
    assert_eq!(na["error"], "repo not found");
}

#[test]
fn sediment_kb_add_repo_json_duplicate_returns_error_json() {
    with_sediment_kb_cache(|| {
        ensure_uncategorized().expect("ensure");
        sediment_kb_add_repo_json(json!({ "name": "dup" })).expect("first");
        let v = sediment_kb_add_repo_json(json!({ "name": "dup" })).expect("dup");
        assert_eq!(v["code"], "duplicate");
    });
}

#[test]
fn sediment_kb_description_source_constraints_hold() {
    crate::services::sediment_kb::validate_description_source_constraints();
}

