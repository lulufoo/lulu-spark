use serde_json::json;

use crate::commands::read::{sediment_kb_categories_json, sediment_kb_repos_json};
use crate::services::sediment_kb::{
    add_category, add_repo, ensure_uncategorized, set_test_repo_validator, UNCATEGORIZED_ID,
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
        f(&cache);
    });
}

fn ok_validator(full_name: &str) -> Result<String, crate::services::sediment_kb::SedimentKbError> {
    Ok(full_name.to_string())
}

#[test]
fn get_sediment_kb_categories_returns_list() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_categories_json().expect("categories");
        let cats = v["categories"].as_array().expect("array");
        assert!(cats.iter().any(|c| c["id"] == UNCATEGORIZED_ID));
    });
}

#[test]
fn get_sediment_kb_repos_resolves_category_names() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Research").expect("cat");
        add_repo("acme/demo", Some(&cat_id)).expect("add");
        let v = sediment_kb_repos_json().expect("repos");
        let repos = v["repos"].as_array().expect("array");
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0]["full_name"], "acme/demo");
        assert_eq!(repos[0]["category_id"], cat_id);
        assert_eq!(repos[0]["category_name"], "Research");
        set_test_repo_validator(None);
    });
}

#[test]
fn get_sediment_kb_repos_empty_by_default() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let v = sediment_kb_repos_json().expect("repos");
        assert_eq!(v["repos"], json!([]));
    });
}
