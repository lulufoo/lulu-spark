use std::fs;

use crate::services::knowledge::resolve_knowledge_for_open;
use crate::services::spark_data::{self, register_knowledge_doc};
use crate::test_support::TestSandbox;

fn plant(sandbox: &TestSandbox, rel: &str) -> std::path::PathBuf {
    let file = sandbox.knowledge_root().join(rel);
    fs::create_dir_all(file.parent().unwrap()).expect("mkdir");
    fs::write(&file, "# doc").expect("write");
    file
}

fn register(repo: &str, rel: &str) -> String {
    register_knowledge_doc(&spark_data::open().expect("db"), repo, rel).expect("register")
}

#[test]
fn resolves_id_to_repo_and_relative_path_without_absolute_path() {
    let sandbox = TestSandbox::new();
    plant(&sandbox, "demo/guide/intro.md");
    let id = register("demo", "guide/intro.md");
    let out = resolve_knowledge_for_open(&id);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["id"], id);
    assert_eq!(out["repo"], "demo");
    assert_eq!(out["path"], "guide/intro.md");
    let text = out.to_string();
    assert!(
        !text.contains(sandbox.cache_dir().to_string_lossy().as_ref()),
        "must not leak an absolute path: {text}"
    );
    assert!(!text.contains("# doc"), "must not return the body: {text}");
}

#[test]
fn unknown_id_is_an_error() {
    let _sandbox = TestSandbox::new();
    let out = resolve_knowledge_for_open("ffffffffffffffffffffffffffffffff");
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"], "Unknown document id");
}

#[test]
fn the_old_hash_style_ids_no_longer_resolve() {
    let sandbox = TestSandbox::new();
    plant(&sandbox, "demo/old.md");
    register("demo", "old.md");
    let out = resolve_knowledge_for_open("demo__old_md");
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"], "Unknown document id");
}

#[test]
fn deleted_file_is_an_error() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "demo/gone.md");
    let id = register("demo", "gone.md");
    fs::remove_file(&file).expect("remove");
    let out = resolve_knowledge_for_open(&id);
    assert_eq!(out["ok"], false, "{out}");
    assert_eq!(out["error"], "File not found");
}

#[test]
fn a_renamed_document_keeps_opening_under_the_same_id() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "demo/a.md");
    let id = register("demo", "a.md");
    fs::rename(&file, file.with_file_name("b.md")).expect("rename");
    spark_data::move_knowledge_paths(&spark_data::open().expect("db"), "demo", "a.md", "b.md")
        .expect("move record");
    let out = resolve_knowledge_for_open(&id);
    assert_eq!(out["ok"], true, "{out}");
    assert_eq!(out["path"], "b.md");
}
