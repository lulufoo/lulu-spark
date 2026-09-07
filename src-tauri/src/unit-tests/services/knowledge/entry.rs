use super::*;
use crate::test_support::TestSandbox;
use std::fs;

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
                "lulufoo/myrepo".into(),
                "docs".into(),
                "guide".into(),
                "file".into(),
            );
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "docs/guide.md");
            assert_eq!(fs::read_to_string(kb.join("myrepo/docs/guide.md")).expect("r"), "");
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
                "lulufoo/myrepo".into(),
                "".into(),
                "notes".into(),
                "dir".into(),
            );
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "notes");
            assert!(kb.join("myrepo/notes").is_dir());
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
                "lulufoo/myrepo".into(),
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
                "lulufoo/myrepo".into(),
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
fn kb_delete_removes_file_and_annotation() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs")).expect("mkdir");
            fs::write(repo_dir.join("docs/a.md"), "hello").expect("w");
            let ann = repo_dir.join(".knowledge_annotations/docs");
            fs::create_dir_all(&ann).expect("ann");
            fs::write(ann.join("a.json"), "{\"ok\":true}").expect("ann w");
        },
        |cfg_dir, kb| {
            let v = kb_delete(cfg_dir, "lulufoo/myrepo".into(), "docs/a.md".into());
            assert_eq!(v["ok"], true);
            let repo_dir = kb.join("myrepo");
            assert!(!repo_dir.join("docs/a.md").exists());
            assert!(!repo_dir.join(".knowledge_annotations/docs/a.json").exists());
        },
    );
}

#[test]
fn kb_delete_removes_directory_and_annotation_tree() {
    with_kb_repo(
        |_, kb| {
            let repo_dir = kb.join("myrepo");
            fs::create_dir_all(repo_dir.join("docs/nested")).expect("mkdir");
            fs::write(repo_dir.join("docs/nested/x.md"), "x").expect("w");
            let ann = repo_dir.join(".knowledge_annotations/docs/nested");
            fs::create_dir_all(&ann).expect("ann");
            fs::write(ann.join("x.json"), "{}").expect("ann w");
            fs::write(repo_dir.join(".knowledge_annotations/docs.json"), "{\"dir\":1}").expect("dir ann");
        },
        |cfg_dir, kb| {
            let v = kb_delete(cfg_dir, "lulufoo/myrepo".into(), "docs".into());
            assert_eq!(v["ok"], true);
            let repo_dir = kb.join("myrepo");
            assert!(!repo_dir.join("docs").exists());
            assert!(!repo_dir.join(".knowledge_annotations/docs").exists());
            assert!(!repo_dir.join(".knowledge_annotations/docs.json").exists());
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
            let v = kb_delete(cfg_dir, "lulufoo/myrepo".into(), "../x.md".into());
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
                "lulufoo/myrepo".into(),
                "".into(),
                "a/b.md".into(),
                "file".into(),
            );
            assert_eq!(v["error"], "invalid name");
            assert_eq!(v["_status"], 400);
        },
    );
}
