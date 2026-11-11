use serde_json::json;
use std::fs;

use super::*;
use crate::services::docs::{create_titled_doc, doc_file_path};
use crate::services::spark_data::{find_doc, open};
use crate::test_support::{block_note_writes, default_inbox_id, default_workspace, default_workspace_dir, TestSandbox};

fn create_note() -> String {
    let created = create_titled_doc(&json!({
        "workspace_id": default_workspace().id,
        "project_id": default_inbox_id(),
        "title": "Original title",
        "filename": "original.md",
        "content": "# 正文标题\n\n正文保持不变。",
        "source_type": "jot",
        "created_at": "202611111230"
    }));
    assert_eq!(created["ok"], json!(true), "{created}");
    created["id"].as_str().expect("id").to_string()
}

#[test]
fn updates_only_title_and_reads_it_back() {
    let _sandbox = TestSandbox::new();
    let id = create_note();
    let before = find_doc(&open().expect("db"), &id).expect("lookup").expect("document").doc;
    let path = doc_file_path(&default_workspace_dir(), "raw", &before.project_id, &before.raw_path).expect("path");
    let body = fs::read_to_string(&path).expect("read original");

    let result = set_doc_title(&id, "  新标题  ");
    assert_eq!(result["ok"], json!(true), "{result}");
    assert_eq!(result["title"], json!("新标题"));
    let after = find_doc(&open().expect("db"), &id).expect("lookup").expect("document").doc;
    assert_eq!(after.title.as_deref(), Some("新标题"));
    assert_eq!(after.raw_path, before.raw_path);
    assert_eq!(after.created_at, before.created_at);
    assert_eq!(after.version, before.version);
    assert_eq!(fs::read_to_string(path).expect("read body"), body);
}

#[test]
fn rejects_blank_invalid_and_missing_documents_without_changes() {
    let _sandbox = TestSandbox::new();
    let id = create_note();
    assert_eq!(set_doc_title(&id, " \n ")["_status"], json!(400));
    assert_eq!(set_doc_title("invalid", "Title")["_status"], json!(400));
    assert_eq!(set_doc_title("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "Title")["_status"], json!(404));
    let doc = find_doc(&open().expect("db"), &id).expect("lookup").expect("document").doc;
    assert_eq!(doc.title.as_deref(), Some("Original title"));
}

#[test]
fn write_failure_does_not_report_success_or_change_title() {
    let _sandbox = TestSandbox::new();
    let id = create_note();
    block_note_writes("UPDATE");
    let result = set_doc_title(&id, "New title");
    assert_eq!(result["_status"], json!(500), "{result}");
    assert_ne!(result["ok"], json!(true));
    let doc = find_doc(&open().expect("db"), &id).expect("lookup").expect("document").doc;
    assert_eq!(doc.title.as_deref(), Some("Original title"));
}

#[test]
fn command_is_registered_in_generate_handler() {
    let lib = fs::read_to_string(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("lib.rs");
    assert!(lib.contains("commands::write::set_doc_title"));
}
