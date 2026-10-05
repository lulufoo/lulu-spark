use super::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use serde_json::json;

use crate::config::paths;
use crate::test_support::TestSandbox;

fn with_sediment_kb_cache<F: FnOnce(&Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let data = sandbox.data_dir();
    f(data.as_path());
}

fn seed_dir(name: &str) {
    add_directory(name).expect("seed");
}

#[test]
fn init_creates_categories_and_repos_with_uncategorized() {
    with_sediment_kb_cache(|wb_root| {
        ensure_uncategorized().expect("ensure");
        let cat_path = paths::sediment_kb_categories_path().expect("cat path");
        let repo_path = paths::sediment_kb_repos_path().expect("repo path");
        assert_eq!(cat_path, wb_root.join("knowledge").join("categories.json"));
        assert_eq!(repo_path, wb_root.join("knowledge").join("repos.json"));
        let cache = paths::cache_dir().expect("cache");
        assert!(!cat_path.starts_with(cache.join("knowledge")));
        assert!(!repo_path.starts_with(cache.join("knowledge")));
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
fn add_directory_creates_record_and_folder() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        add_directory("topic-notes").expect("add");
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].full_name, "topic-notes");
        assert_eq!(repos.repos[0].description, "");
        assert_eq!(repos.repos[0].category_id, UNCATEGORIZED_ID);
        let dir = paths::knowledge_root().expect("kb").join("topic-notes");
        assert!(dir.is_dir());
    });
}

#[test]
fn add_directory_duplicate_rejected() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        seed_dir("demo");
        let err = add_directory("demo").expect_err("dup");
        assert!(err.is_duplicate());
    });
}

#[test]
fn add_directory_rejects_slash_name() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let err = add_directory("acme/demo").expect_err("slash");
        assert!(err.is_invalid_format());
    });
}

#[test]
fn add_directory_then_update_category() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("AI").expect("add cat");
        seed_dir("demo");
        update_repo_category("demo", &cat_id).expect("cat");
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].category_id, cat_id);
    });
}

#[test]
fn remove_repo_removes_entry() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        seed_dir("demo");
        remove_repo("demo").expect("remove");
        assert!(load_repos().expect("load").repos.is_empty());
    });
}

#[test]
fn remove_repo_does_not_delete_clone_dir() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        seed_dir("demo");
        let clone = paths::knowledge_root().expect("kb").join("demo");
        fs::write(clone.join("keep.md"), "x").expect("write");
        remove_repo("demo").expect("remove");
        assert!(load_repos().expect("load").repos.is_empty());
        assert!(clone.join("keep.md").is_file());
    });
}

#[test]
fn update_repo_category_changes_category() {
    with_sediment_kb_cache(|_| {
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Docs").expect("cat");
        seed_dir("demo");
        update_repo_category("demo", &cat_id).expect("update");
        assert_eq!(
            load_repos().expect("load").repos[0].category_id,
            cat_id
        );
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
        ensure_uncategorized().expect("ensure");
        let cat_id = add_category("Temp").expect("cat");
        seed_dir("a");
        seed_dir("b");
        update_repo_category("a", &cat_id).expect("a");
        update_repo_category("b", &cat_id).expect("b");
        remove_category(&cat_id).expect("remove cat");
        let repos = load_repos().expect("load");
        assert!(repos.repos.iter().all(|r| r.category_id == UNCATEGORIZED_ID));
        let cats = load_categories().expect("cats");
        assert!(!cats.categories.iter().any(|c| c.id == cat_id));
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
        ensure_uncategorized().expect("ensure");
        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let counter = Arc::clone(&counter);
            handles.push(thread::spawn(move || {
                let n = counter.fetch_add(1, Ordering::SeqCst);
                add_directory(&format!("repo{n}")).expect("add");
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
    });
}

#[test]
fn service_source_uses_paths_ssot_not_hardcoded_cache() {
    validate_paths_ssot_constraints();
}
