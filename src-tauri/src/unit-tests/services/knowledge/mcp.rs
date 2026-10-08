use super::*;
use std::fs;

use crate::services::knowledge::doc_map;
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

#[test]
fn remember_knowledge_doc_issues_id() {
    let sandbox = TestSandbox::new();
    let file = sandbox.cache_dir().join("open.md");
    fs::write(&file, "body").expect("write");
    let first = remember_knowledge_doc_value(&file.to_string_lossy());
    assert_eq!(first["ok"], true);
    let id = first["id"].as_str().expect("id");
    assert_eq!(id.len(), 12);
    let second = remember_knowledge_doc_value(&file.to_string_lossy());
    assert_eq!(second["id"], id);
    let empty = remember_knowledge_doc_value("  ");
    assert_eq!(empty["error"], "Missing path");
    assert_eq!(empty["_status"], 400);
}

#[test]
fn get_unknown_id_fails() {
    let _sandbox = TestSandbox::new();
    let v = get_knowledge_path_by_id("not-issued");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"], "Unknown document id");
}

#[test]
fn get_remembered_id_returns_path_without_body() {
    let sandbox = TestSandbox::new();
    let file = sandbox.cache_dir().join("kb.md");
    fs::write(&file, "secret body").expect("write");
    let id = doc_map::remember_path(&file).expect("remember");
    let v = get_knowledge_path_by_id(&id);
    assert_eq!(v["ok"], true);
    assert_eq!(v["id"], id);
    assert!(v.get("content").is_none());
    let path = v["path"].as_str().expect("path");
    assert!(path.ends_with("kb.md"));
    assert!(!v.to_string().contains("secret body"));
}
