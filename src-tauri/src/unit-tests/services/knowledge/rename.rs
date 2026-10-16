use super::*;
use crate::services::spark_data::{
    self, find_knowledge_doc_id, get_knowledge_doc, register_knowledge_doc, update_knowledge_doc,
};
use crate::test_support::TestSandbox;
use serde_json::json;
use std::fs;

/// Register `(myrepo, path)` with one comment; returns its `doc_id`.
fn seed_commented_doc(path: &str) -> String {
    let conn = spark_data::open().expect("db");
    let id = register_knowledge_doc(&conn, "myrepo", path).expect("register");
    let mut row = get_knowledge_doc(&conn, &id).expect("get").expect("row");
    row.comments = json!([{ "id": "c1", "text": "keep me", "ts": "t" }]);
    update_knowledge_doc(&conn, &row).expect("update");
    id
}

fn doc_id_at(path: &str) -> Option<String> {
    let conn = spark_data::open().expect("db");
    find_knowledge_doc_id(&conn, "myrepo", path).expect("find")
}

fn comments_of(id: &str) -> serde_json::Value {
    let conn = spark_data::open().expect("db");
    get_knowledge_doc(&conn, id).expect("get").expect("row").comments
}

fn with_kb_repo<F: FnOnce(&std::path::Path, &std::path::Path)>(
    setup: impl FnOnce(&std::path::Path, &std::path::Path),
    f: F,
) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let kb = sandbox.knowledge_root();
    setup(cfg_dir, &kb);
    f(cfg_dir, &kb);
}

#[test]
fn kb_rename_moves_file_and_keeps_record_id_and_comments() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("docs/a.md"), "hello").expect("w");
        },
        |cfg_dir, kb| {
            let id = seed_commented_doc("docs/a.md");
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "docs/a.md".into(), "b.md".into());
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "docs/b.md");
            let repo_dir = kb.join("myrepo");
            assert!(!repo_dir.join("docs/a.md").exists());
            assert_eq!(fs::read_to_string(repo_dir.join("docs/b.md")).expect("r"), "hello");
            assert_eq!(doc_id_at("docs/a.md"), None);
            assert_eq!(doc_id_at("docs/b.md").as_deref(), Some(id.as_str()), "id survives a rename");
            assert_eq!(comments_of(&id)[0]["text"], "keep me");
        },
    );
}

#[test]
fn kb_rename_moves_records_below_a_directory_only() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs/nested")).expect("mkdir");
            fs::create_dir_all(repo_dir.join("docs2")).expect("mkdir");
            fs::write(repo_dir.join("docs/nested/x.md"), "x").expect("w");
            fs::write(repo_dir.join("docs/a.md"), "a").expect("w");
            fs::write(repo_dir.join("docs2/y.md"), "y").expect("w");
        },
        |cfg_dir, kb| {
            let nested = seed_commented_doc("docs/nested/x.md");
            let top = seed_commented_doc("docs/a.md");
            let sibling = seed_commented_doc("docs2/y.md");
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "docs".into(), "guides".into());
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "guides");
            assert!(kb.join("myrepo/guides/nested/x.md").is_file());
            assert_eq!(doc_id_at("guides/nested/x.md").as_deref(), Some(nested.as_str()));
            assert_eq!(doc_id_at("guides/a.md").as_deref(), Some(top.as_str()));
            assert_eq!(doc_id_at("docs/a.md"), None);
            assert_eq!(doc_id_at("docs2/y.md").as_deref(), Some(sibling.as_str()));
        },
    );
}

#[test]
fn kb_rename_replaces_a_stale_record_at_the_destination() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(&repo_dir).expect("mkdir");
            fs::write(repo_dir.join("a.md"), "a").expect("w");
        },
        |cfg_dir, _| {
            let moving = seed_commented_doc("a.md");
            let stale = seed_commented_doc("b.md");
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "a.md".into(), "b.md".into());
            assert_eq!(v["ok"], true);
            assert_eq!(doc_id_at("b.md").as_deref(), Some(moving.as_str()));
            let conn = spark_data::open().expect("db");
            assert!(get_knowledge_doc(&conn, &stale).expect("get").is_none());
        },
    );
}

#[test]
fn kb_rename_rejects_slash_name() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
            fs::write(kb.join("myrepo/a.md"), "a").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "a.md".into(), "b/c.md".into());
            assert_eq!(v["error"], "invalid name");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_rename_rejects_destination_exists() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(&repo_dir).expect("mkdir");
            fs::write(repo_dir.join("a.md"), "a").expect("w");
            fs::write(repo_dir.join("b.md"), "b").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "a.md".into(), "b.md".into());
            assert_eq!(v["error"], "destination exists");
            assert_eq!(v["_status"], 409);
        },
    );
}

#[test]
fn kb_rename_same_name_is_noop() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
            fs::write(kb.join("myrepo/a.md"), "a").expect("w");
        },
        |cfg_dir, kb| {
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "a.md".into(), "a.md".into());
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "a.md");
            assert_eq!(fs::read_to_string(kb.join("myrepo/a.md")).expect("r"), "a");
        },
    );
}

#[test]
fn kb_rename_rejects_missing_source() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "gone.md".into(), "a.md".into());
            assert_eq!(v["error"], "not found: gone.md");
            assert_eq!(v["_status"], 404);
        },
    );
}

#[test]
fn kb_rename_rejects_traversal_path() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_rename(
                cfg_dir,
                "lulufoo/myrepo".into(),
                "../x.md".into(),
                "a.md".into(),
            );
            assert_eq!(v["error"], "invalid path");
            assert_eq!(v["_status"], 400);
        },
    );
}
