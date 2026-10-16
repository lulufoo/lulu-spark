use super::*;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use crate::config::paths;
use crate::services::spark_data::{self, list_knowledge_directories, UNCATEGORIZED};
use crate::test_support::TestSandbox;

fn with_sediment_kb_cache<F: FnOnce(&Path)>(f: F) {
    let sandbox = TestSandbox::new();
    f(sandbox.data_dir().as_path());
}

fn seed_dir(name: &str) {
    add_directory(name).expect("seed");
}

fn table_folders() -> Vec<String> {
    let conn = spark_data::open().expect("db");
    list_knowledge_directories(&conn)
        .expect("rows")
        .into_iter()
        .map(|r| r.folder)
        .collect()
}

#[test]
fn init_does_not_write_json_and_lists_uncategorized() {
    with_sediment_kb_cache(|wb_root| {
        ensure_uncategorized().expect("ensure");
        let cats = load_categories().expect("cats");
        assert_eq!(cats.categories.len(), 1);
        assert_eq!(cats.categories[0].id, UNCATEGORIZED_ID);
        assert_eq!(cats.categories[0].name, UNCATEGORIZED_NAME);
        assert!(load_repos().expect("repos").repos.is_empty());
        assert!(!wb_root.join("knowledge").join("repos.json").is_file());
        assert!(!wb_root.join("knowledge").join("categories.json").is_file());
    });
}

#[test]
fn add_directory_creates_record_and_folder() {
    with_sediment_kb_cache(|_| {
        add_directory("topic-notes").expect("add");
        let repos = load_repos().expect("load");
        assert_eq!(repos.repos[0].full_name, "topic-notes");
        assert_eq!(repos.repos[0].description, "");
        assert_eq!(repos.repos[0].category_id, UNCATEGORIZED);
        assert!(paths::knowledge_root().expect("kb").join("topic-notes").is_dir());
        assert_eq!(table_folders(), vec!["topic-notes"]);
    });
}

#[test]
fn add_directory_duplicate_rejected() {
    with_sediment_kb_cache(|_| {
        seed_dir("demo");
        let err = add_directory("demo").expect_err("dup");
        assert!(err.is_duplicate());
    });
}

#[test]
fn add_directory_rejects_slash_name() {
    with_sediment_kb_cache(|_| {
        let err = add_directory("acme/demo").expect_err("slash");
        assert!(err.is_invalid_format());
    });
}

#[test]
fn add_directory_then_update_category() {
    with_sediment_kb_cache(|_| {
        seed_dir("demo");
        update_repo_category("demo", "AI").expect("cat");
        assert_eq!(load_repos().expect("load").repos[0].category_id, "AI");
        let cats = load_categories().expect("cats");
        assert!(cats.categories.iter().any(|c| c.name == "AI"));
        assert!(cats.categories.iter().any(|c| c.name == UNCATEGORIZED_NAME));
    });
}

#[test]
fn remove_repo_removes_entry() {
    with_sediment_kb_cache(|_| {
        seed_dir("demo");
        remove_repo("demo").expect("remove");
        assert!(load_repos().expect("load").repos.is_empty());
    });
}

#[test]
fn remove_repo_does_not_delete_clone_dir() {
    with_sediment_kb_cache(|_| {
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
        seed_dir("demo");
        update_repo_category("demo", "Docs").expect("update");
        assert_eq!(load_repos().expect("load").repos[0].category_id, "Docs");
    });
}

#[test]
fn add_category_and_rename() {
    with_sediment_kb_cache(|_| {
        seed_dir("demo");
        let id = add_category("Research").expect("add");
        update_repo_category("demo", &id).expect("assign");
        rename_category(&id, "R&D").expect("rename");
        let cats = load_categories().expect("load");
        assert!(cats.categories.iter().any(|c| c.name == "R&D"));
        assert!(!cats.categories.iter().any(|c| c.name == "Research"));
        assert_eq!(load_repos().expect("load").repos[0].category_id, "R&D");
    });
}

#[test]
fn add_category_empty_name_rejected() {
    with_sediment_kb_cache(|_| {
        let err = add_category("  ").expect_err("empty");
        assert!(err.is_invalid_name());
    });
}

#[test]
fn remove_category_reassigns_repos_to_uncategorized() {
    with_sediment_kb_cache(|_| {
        let cat_id = add_category("Temp").expect("cat");
        seed_dir("a");
        seed_dir("b");
        update_repo_category("a", &cat_id).expect("a");
        update_repo_category("b", &cat_id).expect("b");
        remove_category(&cat_id).expect("remove cat");
        assert!(load_repos()
            .expect("load")
            .repos
            .iter()
            .all(|r| r.category_id == UNCATEGORIZED_ID));
        let cats = load_categories().expect("cats");
        assert!(!cats.categories.iter().any(|c| c.id == cat_id));
    });
}

#[test]
fn remove_uncategorized_rejected() {
    with_sediment_kb_cache(|_| {
        let err = remove_category(UNCATEGORIZED_ID).expect_err("protected");
        assert!(err.is_protected_category());
    });
}

#[test]
fn concurrent_writes_do_not_lose_directories() {
    with_sediment_kb_cache(|_| {
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
        assert_eq!(load_repos().expect("load").repos.len(), 8);
        assert_eq!(table_folders().len(), 8);
    });
}

#[test]
fn service_source_uses_directory_table_not_json() {
    let src = include_str!("../../services/sediment_kb.rs");
    assert!(src.contains("knowledge_directory"));
    assert!(src.contains("spark_data::open"));
    assert!(!src.contains("atomic_json::write_json"));
}

#[test]
fn description_source_constraints_hold() {
    let sediment_prod = include_str!("../../services/sediment_kb.rs");
    let write_cmd = include_str!("../../commands/write.rs");
    let spark_read = include_str!("../../services/spark_read/config.rs");
    assert!(!sediment_prod.contains("validate_repo_access"));
    assert!(!sediment_prod.contains("integrations::github"));
    for (label, src) in [
        ("sediment_kb", sediment_prod),
        ("write", write_cmd),
        ("spark_read", spark_read),
    ] {
        for forbidden in ["README", "frontmatter"] {
            assert!(!src.contains(forbidden), "{label} must not use {forbidden}");
        }
    }
    let get_topics = spark_read
        .split("pub fn get_topics")
        .nth(1)
        .and_then(|s| s.split("\npub fn ").next())
        .expect("get_topics");
    assert!(!get_topics.contains("index.json"));
}
