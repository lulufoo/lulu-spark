use super::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use serde_json::json;

use crate::config::paths;
use crate::test_support::with_test_config_dir;

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

fn ok_validator(full_name: &str) -> Result<String, SedimentKbError> {
    Ok(full_name.to_string())
}

fn inaccessible_validator(_full_name: &str) -> Result<String, SedimentKbError> {
    Err(SedimentKbError::not_accessible("repo not found"))
}

#[test]
fn init_creates_categories_and_repos_with_uncategorized() {
    with_sediment_kb_cache(|cache| {
        ensure_uncategorized().expect("ensure");
        let cat_path = paths::sediment_kb_categories_path().expect("cat path");
        let repo_path = paths::sediment_kb_repos_path().expect("repo path");
        assert!(cat_path.starts_with(cache.join("sediment-kb")));
        assert!(repo_path.starts_with(cache.join("sediment-kb")));
        assert!(cat_path.is_file());
        assert!(repo_path.is_file());

        let cats = load_categories().expect("load cats");
        assert_eq!(cats.version, 1);
        assert!(
            cats.categories
                .iter()
                .any(|c| c.id == UNCATEGORIZED_ID && c.name == "未分类")
        );

        let repos = load_repos().expect("load repos");
        assert_eq!(repos.version, 1);
        assert!(repos.repos.is_empty());
    });
}

#[test]
fn add_repo_accepts_owner_slash_repo() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        add_repo("acme/demo", None).expect("add");
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos.len(), 1);
        assert_eq!(repos.repos[0].full_name, "acme/demo");
        assert_eq!(repos.repos[0].category_id, UNCATEGORIZED_ID);
        set_test_repo_validator(None);
    });
}

#[test]
fn add_repo_accepts_github_url() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        add_repo("https://github.com/acme/demo/", None).expect("add");
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].full_name, "acme/demo");
        set_test_repo_validator(None);
    });
}

#[test]
fn add_repo_with_category() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("AI").expect("add cat");
        add_repo("acme/demo", Some(&cat_id)).expect("add");
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].category_id, cat_id);
        set_test_repo_validator(None);
    });
}

#[test]
fn add_repo_duplicate_rejected() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        add_repo("acme/demo", None).expect("first");
        let err = add_repo("acme/demo", None).expect_err("dup");
        assert!(err.is_duplicate());
        set_test_repo_validator(None);
    });
}

#[test]
fn add_repo_invalid_format_rejected() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let err = add_repo("not-a-repo", None).expect_err("invalid");
        assert!(err.is_invalid_format());
        set_test_repo_validator(None);
    });
}

#[test]
fn add_repo_not_accessible_rejected() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(inaccessible_validator));
        ensure_uncategorized().expect("ensure");
        let err = add_repo("acme/missing", None).expect_err("missing");
        assert!(err.is_not_accessible());
        set_test_repo_validator(None);
    });
}

#[test]
fn remove_repo_removes_entry() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        add_repo("acme/demo", None).expect("add");
        super::remove_repo("acme/demo").expect("remove");
        assert!(load_repos().expect("load").repos.is_empty());
        set_test_repo_validator(None);
    });
}

#[test]
fn update_repo_category_changes_category() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Docs").expect("cat");
        add_repo("acme/demo", None).expect("add");
        super::update_repo_category("acme/demo", &cat_id).expect("update");
        assert_eq!(
            load_repos().expect("load").repos[0].category_id,
            cat_id
        );
        set_test_repo_validator(None);
    });
}

#[test]
fn add_category_and_rename() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let id = add_category("Research").expect("add");
        rename_category(&id, "R&D").expect("rename");
        let cats = load_categories().expect("load");
        let cat = cats
            .categories
            .iter()
            .find(|c| c.id == id)
            .expect("found");
        assert_eq!(cat.name, "R&D");
    });
}

#[test]
fn add_category_empty_name_rejected() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let err = add_category("  ").expect_err("empty");
        assert!(err.is_invalid_name());
    });
}

#[test]
fn remove_category_reassigns_repos_to_uncategorized() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Temp").expect("cat");
        add_repo("acme/a", Some(&cat_id)).expect("a");
        add_repo("acme/b", Some(&cat_id)).expect("b");
        remove_category(&cat_id).expect("remove cat");
        let repos = load_repos().expect("load");
        assert!(repos.repos.iter().all(|r| r.category_id == UNCATEGORIZED_ID));
        let cats = load_categories().expect("cats");
        assert!(!cats.categories.iter().any(|c| c.id == cat_id));
        set_test_repo_validator(None);
    });
}

#[test]
fn remove_uncategorized_rejected() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let err = remove_category(UNCATEGORIZED_ID).expect_err("protected");
        assert!(err.is_protected_category());
    });
}

#[test]
fn concurrent_writes_do_not_corrupt_json() {
    with_sediment_kb_cache(|_| {
        set_test_repo_validator(Some(ok_validator));
        ensure_uncategorized().expect("ensure");
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let counter = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                let n = counter.fetch_add(1, Ordering::SeqCst);
                let name = format!("org/repo{n}");
                add_repo(&name, None).expect("add");
            }));
        }
        for h in handles {
            h.join().expect("join");
        }
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos.len(), 8);
        let text = fs::read_to_string(paths::sediment_kb_repos_path().expect("path"))
            .expect("read");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["version"], json!(1));
        set_test_repo_validator(None);
    });
}
