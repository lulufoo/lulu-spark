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
fn kb_rename_moves_file_and_annotation() {
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
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "docs/a.md".into(), "b.md".into());
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "docs/b.md");
            let repo_dir = kb.join("myrepo");
            assert!(!repo_dir.join("docs/a.md").exists());
            assert_eq!(fs::read_to_string(repo_dir.join("docs/b.md")).expect("r"), "hello");
            assert!(!repo_dir.join(".knowledge_annotations/docs/a.json").exists());
            assert_eq!(
                fs::read_to_string(repo_dir.join(".knowledge_annotations/docs/b.json")).expect("ann"),
                "{\"ok\":true}"
            );
        },
    );
}

#[test]
fn kb_rename_moves_directory_and_annotation_tree() {
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
            let v = kb_rename(cfg_dir, "lulufoo/myrepo".into(), "docs".into(), "guides".into());
            assert_eq!(v["ok"], true);
            assert_eq!(v["path"], "guides");
            let repo_dir = kb.join("myrepo");
            assert!(!repo_dir.join("docs").exists());
            assert!(repo_dir.join("guides/nested/x.md").is_file());
            assert!(repo_dir.join(".knowledge_annotations/guides/nested/x.json").is_file());
            assert_eq!(
                fs::read_to_string(repo_dir.join(".knowledge_annotations/guides.json")).expect("dir ann"),
                "{\"dir\":1}"
            );
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
