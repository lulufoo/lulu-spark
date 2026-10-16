use super::*;
use std::fs;

use crate::services::spark_data::{self, register_knowledge_doc};
use crate::services::sediment_kb::{self, UNCATEGORIZED_ID};
use crate::test_support::TestSandbox;

#[test]
fn list_categories_includes_uncategorized() {
    let _sandbox = TestSandbox::new();
    sediment_kb::ensure_uncategorized().expect("ensure");
    let v = list_knowledge_categories_value();
    let rows = v.as_array().expect("array");
    assert!(rows.iter().any(|r| r["id"] == UNCATEGORIZED_ID && r["name"] == "Uncategorized"));
}

#[test]
fn list_repos_uses_sediment_rows_without_inbox() {
    let _sandbox = TestSandbox::new();
    sediment_kb::ensure_uncategorized().expect("ensure");
    sediment_kb::add_directory("demo").expect("add");
    let v = list_knowledge_repos_value();
    let rows = v.as_array().expect("array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["full_name"], "demo");
    assert_eq!(rows[0]["description"], "");
    assert_eq!(rows[0]["category_id"], UNCATEGORIZED_ID);
    assert!(!rows.iter().any(|r| r.get("inbox").is_some()));
}

#[test]
fn search_missing_q_is_400() {
    let sandbox = TestSandbox::new();
    let v = search_knowledge_mcp(sandbox.config_dir(), "  ", None);
    assert_eq!(v["error"], "Missing q");
    assert_eq!(v["_status"], 400);
}

fn plant(sandbox: &TestSandbox, rel: &str, body: &str) -> std::path::PathBuf {
    let file = sandbox.knowledge_root().join(rel);
    fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    fs::write(&file, body).expect("write");
    file
}

fn register(repo: &str, rel: &str) -> String {
    register_knowledge_doc(&spark_data::open().expect("db"), repo, rel).expect("register")
}

#[test]
fn remember_knowledge_doc_registers_once_and_reuses_the_id() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "demo/docs/open.md", "body");
    let first = remember_knowledge_doc_value(&file.to_string_lossy());
    assert_eq!(first["ok"], true, "{first}");
    let id = first["id"].as_str().expect("id");
    assert_eq!(id.len(), 32);
    assert_eq!(remember_knowledge_doc_value(&file.to_string_lossy())["id"], id);
    assert_eq!(id, register("demo", "docs/open.md"), "same record as the registry's");
    let empty = remember_knowledge_doc_value("  ");
    assert_eq!(empty["error"], "Missing path");
    assert_eq!(empty["_status"], 400);
}

#[test]
fn remember_knowledge_doc_rejects_files_outside_the_library() {
    let sandbox = TestSandbox::new();
    let outside = sandbox.cache_dir().join("elsewhere.md");
    fs::write(&outside, "x").expect("write");
    let out = remember_knowledge_doc_value(&outside.to_string_lossy());
    assert_eq!(out["error"], "Not in the knowledge library");
    assert_eq!(out["_status"], 400);
    let loose = plant(&sandbox, "loose.md", "x");
    assert_eq!(
        remember_knowledge_doc_value(&loose.to_string_lossy())["error"],
        "Not in the knowledge library",
        "a file needs a repo directory"
    );
}

#[test]
fn knowledge_doc_id_registers_from_folder_and_rel() {
    let sandbox = TestSandbox::new();
    plant(&sandbox, "demo/docs/open.md", "body");
    let first = knowledge_doc_id_value("demo", "docs/open.md");
    assert_eq!(first["ok"], true, "{first}");
    let id = first["id"].as_str().expect("id");
    assert_eq!(id.len(), 32);
    assert_eq!(knowledge_doc_id_value("demo", "docs/open.md")["id"], id);
    assert_eq!(id, register("demo", "docs/open.md"));
    assert_eq!(knowledge_doc_id_value("demo", "")["error"], "Missing path");
}

#[test]
fn get_unknown_id_fails() {
    let _sandbox = TestSandbox::new();
    let v = get_knowledge_path_by_id("not-issued");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "Unknown document id");
}

#[test]
fn get_registered_id_returns_path_without_body() {
    let sandbox = TestSandbox::new();
    plant(&sandbox, "demo/kb.md", "secret body");
    let id = register("demo", "kb.md");
    let v = get_knowledge_path_by_id(&id);
    assert_eq!(v["ok"], true);
    assert_eq!(v["id"], id);
    assert!(v.get("content").is_none());
    let path = v["path"].as_str().expect("path");
    assert!(path.ends_with("demo/kb.md"));
    assert!(!v.to_string().contains("secret body"));
}

#[test]
fn get_id_of_a_vanished_file_is_file_not_found() {
    let sandbox = TestSandbox::new();
    let file = plant(&sandbox, "demo/gone.md", "x");
    let id = register("demo", "gone.md");
    fs::remove_file(&file).expect("remove");
    let v = get_knowledge_path_by_id(&id);
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "File not found");
}
