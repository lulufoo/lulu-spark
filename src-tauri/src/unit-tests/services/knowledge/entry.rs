use super::*;
use crate::services::spark_data::{
    self, find_knowledge_doc_id, get_knowledge_doc, register_knowledge_doc, update_knowledge_doc,
};
use crate::test_support::TestSandbox;
use serde_json::json;
use std::fs;

/// Register `(myrepo, path)` and give it one comment so deletion is observable.
fn seed_commented_doc(path: &str) -> String {
    let conn = spark_data::open().expect("db");
    let id = register_knowledge_doc(&conn, "myrepo", path).expect("register");
    let mut row = get_knowledge_doc(&conn, &id).expect("get").expect("row");
    row.comments = json!([{ "id": "c1", "text": "keep me", "ts": "t" }]);
    update_knowledge_doc(&conn, &row).expect("update");
    id
}

fn doc_exists(path: &str) -> bool {
    let conn = spark_data::open().expect("db");
    find_knowledge_doc_id(&conn, "myrepo", path).expect("find").is_some()
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
fn kb_create_file_appends_md_and_writes_empty() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo/docs")).expect("mkdir");
        },
        |cfg_dir, kb| {
            let v = kb_create(
                cfg_dir,
                "myrepo".into(),
                "docs".into(),
                "guide".into(),
                "file".into(),
            );
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "docs/guide.md");
            assert_eq!(fs::read_to_string(kb.join("myrepo/docs/guide.md")).expect("r"), "");
            assert!(doc_exists("docs/guide.md"), "a created file gets its record");
        },
    );
}

#[test]
fn kb_create_dir_at_repo_root() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, kb| {
            let v = kb_create(
                cfg_dir,
                "myrepo".into(),
                "".into(),
                "notes".into(),
                "dir".into(),
            );
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "notes");
            assert!(kb.join("myrepo/notes").is_dir());
            assert!(!doc_exists("notes"), "folders have no record");
        },
    );
}

#[test]
fn kb_create_rejects_non_md_file() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_create(
                cfg_dir,
                "myrepo".into(),
                "".into(),
                "a.txt".into(),
                "file".into(),
            );
            assert_eq!(v["error"], "file must be .md");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_create_rejects_destination_exists() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
            fs::write(kb.join("myrepo/a.md"), "x").expect("w");
        },
        |cfg_dir, _| {
            let v = kb_create(
                cfg_dir,
                "myrepo".into(),
                "".into(),
                "a.md".into(),
                "file".into(),
            );
            assert_eq!(v["error"], "destination exists");
            assert_eq!(v["_status"], 409);
        },
    );
}

#[test]
fn kb_delete_removes_file_and_its_record() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("docs/a.md"), "hello").expect("w");
            fs::write(repo_dir.join("docs/b.md"), "other").expect("w");
        },
        |cfg_dir, kb| {
            seed_commented_doc("docs/a.md");
            seed_commented_doc("docs/b.md");
            let v = kb_delete(cfg_dir, "myrepo".into(), "docs/a.md".into());
            assert_eq!(v["ok"], true);
            assert!(!kb.join("myrepo/docs/a.md").exists());
            assert!(!doc_exists("docs/a.md"), "App-side delete removes the record");
            assert!(doc_exists("docs/b.md"), "other records stay");
        },
    );
}

#[test]
fn kb_delete_removes_directory_and_records_below_it_only() {
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
            seed_commented_doc("docs/nested/x.md");
            seed_commented_doc("docs/a.md");
            seed_commented_doc("docs2/y.md");
            let v = kb_delete(cfg_dir, "myrepo".into(), "docs".into());
            assert_eq!(v["ok"], true);
            assert!(!kb.join("myrepo/docs").exists());
            assert!(!doc_exists("docs/nested/x.md"));
            assert!(!doc_exists("docs/a.md"));
            assert!(doc_exists("docs2/y.md"), "a sibling sharing the name prefix must stay");
        },
    );
}

#[test]
fn kb_delete_rejects_traversal_path() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_delete(cfg_dir, "myrepo".into(), "../x.md".into());
            assert_eq!(v["error"], "invalid path");
            assert_eq!(v["_status"], 400);
        },
    );
}

#[test]
fn kb_create_rejects_slash_name() {
    with_kb_repo(
        |_, kb| {
            fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        },
        |cfg_dir, _| {
            let v = kb_create(
                cfg_dir,
                "myrepo".into(),
                "".into(),
                "a/b.md".into(),
                "file".into(),
            );
            assert_eq!(v["error"], "invalid name");
            assert_eq!(v["_status"], 400);
        },
    );
}
