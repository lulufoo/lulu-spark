use serde_json::json;

use super::*;
use crate::services::spark_data::{find_doc, open};
use crate::test_support::TestSandbox;

fn create_summary() -> String {
    let created = crate::services::docs::create_doc_content(&json!({
        "title": "T",
        "content": "# 标题\n\n这是足够长的正文内容。\n",
        "digest": "never",
        "source_type": "summary",
    }));
    created["id"]
        .as_str()
        .expect("id")
        .to_string()
}

fn row_source(id: &str) -> Option<String> {
    find_doc(&open().expect("db"), id)
        .expect("get")
        .expect("row")
        .doc
        .source_type
}

#[test]
fn set_source_type_changes_the_row_and_rejects_unknown_ids() {
    let _sandbox = TestSandbox::new();
    let id = create_summary();
    assert_eq!(row_source(&id).as_deref(), Some("summary"));
    let ok = set_source_type(&id, "dialogue");
    assert_eq!(ok["ok"], json!(true));
    assert_eq!(row_source(&id).as_deref(), Some("dialogue"));

    let bad = set_source_type(&id, "theme-line");
    assert_eq!(bad["_status"], json!(400));
    assert_eq!(row_source(&id).as_deref(), Some("dialogue"));
}

#[test]
fn set_source_type_accepts_hidden_jot_and_rejects_empty() {
    let _sandbox = TestSandbox::new();
    let id = create_summary();
    assert_eq!(set_source_type(&id, "jot")["ok"], json!(true));
    assert_eq!(row_source(&id).as_deref(), Some("jot"));
    assert_eq!(set_source_type(&id, "  ")["_status"], json!(400));
}

#[test]
fn command_is_registered_in_generate_handler() {
    let lib = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("lib.rs");
    assert!(
        lib.contains("commands::write::set_doc_source_type"),
        "lib.rs generate_handler must register set_doc_source_type"
    );
}
