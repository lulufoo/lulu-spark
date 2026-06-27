use serde_json::json;

use crate::commands::write::{
    map_sediment_kb_error, sediment_kb_add_category_json, sediment_kb_add_repo_json,
    sediment_kb_remove_category_json, sediment_kb_remove_repo_json, sediment_kb_rename_category_json,
    sediment_kb_update_repo_category_json,
};
use crate::services::sediment_kb::{
    add_category, add_repo, ensure_uncategorized, load_repos, set_test_repo_validator,
    SedimentKbError, UNCATEGORIZED_ID,
};
use crate::test_support::with_test_config_dir;

use std::fs;
use std::path::Path;

fn with_sediment_kb_cache<F: FnOnce(&Path)>(f: F) {
    with_test_config_dir(|cfg| {
        let cache = cfg.join("cache");
        fs::write(
            cfg.join("config.toml"),
            format!(r#"cache_dir = "{}""#, cache.display()),
        )
        .expect("write config");
        crate::config::secrets::test_secrets_clear();
        f(&cache);
    });
}

fn ok_validator(full_name: &str) -> Result<String, SedimentKbError> {
    Ok(full_name.to_string())
}

fn no_token_validator(_: &str) -> Result<String, SedimentKbError> {
    Err(SedimentKbError::not_accessible("请先在设置中配置 GitHub Token"))
}

#[test]
fn sediment_kb_add_repo_json_defaults_to_uncategorized() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_repo_json(json!({ "full_name": "acme/demo" })).expect("add");
        assert!(v.get("error").is_none());
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].category_id, UNCATEGORIZED_ID);
        assert_eq!(repos.repos[0].description, "");
        set_test_repo_validator(None);
    });
}

#[test]
fn sediment_kb_add_repo_json_saves_description() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_repo_json(json!({
            "full_name": "acme/demo",
            "description": "demo desc"
        }))
        .expect("add");
        assert!(v.get("error").is_none());
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].description, "demo desc");
        set_test_repo_validator(None);
    });
}

#[test]
fn sediment_kb_add_repo_json_saves_description_with_category() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Docs").expect("cat");
        let v = sediment_kb_add_repo_json(json!({
            "full_name": "acme/demo",
            "category_id": cat_id,
            "description": "demo desc"
        }))
        .expect("add");
        assert!(v.get("error").is_none());
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].category_id, cat_id);
        assert_eq!(repos.repos[0].description, "demo desc");
        set_test_repo_validator(None);
    });
}

#[test]
fn sediment_kb_add_repo_json_defaults_missing_description_to_empty_string() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_repo_json(json!({ "full_name": "acme/demo" })).expect("add");
        assert!(v.get("error").is_none());
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].description, "");
        set_test_repo_validator(None);
    });
}

#[test]
fn sediment_kb_add_repo_json_keeps_empty_description() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_repo_json(json!({
            "full_name": "acme/demo",
            "description": ""
        }))
        .expect("add");
        assert!(v.get("error").is_none());
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].description, "");
        set_test_repo_validator(None);
    });
}

#[test]
fn sediment_kb_add_repo_json_requires_full_name_before_description() {
    let err = sediment_kb_add_repo_json(json!({ "description": "demo desc" }))
        .expect_err("missing full_name should be an Err");
    assert_eq!(err, "missing full_name");
}

#[test]
fn sediment_kb_add_repo_json_crud_roundtrip() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        sediment_kb_add_repo_json(json!({ "full_name": "acme/a" })).expect("add");
        let cat_id = add_category("Docs").expect("cat");
        sediment_kb_update_repo_category_json(json!({
            "full_name": "acme/a",
            "category_id": cat_id
        }))
        .expect("update");
        sediment_kb_remove_repo_json(json!({ "full_name": "acme/a" })).expect("remove");
        assert!(load_repos().expect("load").repos.is_empty());
        set_test_repo_validator(None);
    });
}

#[test]
fn sediment_kb_category_crud() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_category_json(json!({ "name": "AI" })).expect("add cat");
        let id = v["id"].as_str().expect("id");
        sediment_kb_rename_category_json(json!({ "id": id, "name": "ML" })).expect("rename");
        set_test_repo_validator(Some(ok_validator));
        add_repo("acme/x", Some(id), "").expect("seed");
        sediment_kb_remove_category_json(json!({ "id": id })).expect("remove cat");
        let repos = load_repos().expect("repos");
        assert!(repos.repos.iter().all(|r| r.category_id == UNCATEGORIZED_ID));
        set_test_repo_validator(None);
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

    let token = map_sediment_kb_error(SedimentKbError::not_accessible(
        "请先在设置中配置 GitHub Token",
    ));
    assert_eq!(token["code"], "no_token");
}

#[test]
fn sediment_kb_add_repo_json_duplicate_returns_error_json() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        sediment_kb_add_repo_json(json!({ "full_name": "acme/dup" })).expect("first");
        let v = sediment_kb_add_repo_json(json!({ "full_name": "acme/dup" })).expect("dup");
        assert_eq!(v["code"], "duplicate");
        set_test_repo_validator(None);
    });
}

#[test]
fn sediment_kb_description_source_constraints_hold() {
    crate::services::sediment_kb::validate_description_source_constraints();
}

#[test]
fn sediment_kb_add_repo_json_no_token_error() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(no_token_validator));
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_add_repo_json(json!({ "full_name": "acme/private" })).expect("err");
        assert_eq!(v["code"], "no_token");
        set_test_repo_validator(None);
    });
}
