use std::fs;

use serde_json::json;

use crate::commands::write::archive_document_json;
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30
> 来源：theme-summary
> 导航：[digest](../../../digest/inbox/test-topic/202606191430-test-slug.md)

---

Summary body here with enough content.
"#;

fn setup_corpus() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(wb.join("raw")).expect("raw dir");
    fs::create_dir_all(wb.join("digest")).expect("digest dir");
    fs::write(wb.join("index.json"), br#"{"entries":{}}"#).expect("index");
    sandbox
}

#[test]
fn archive_document_json_success_returns_entry_handle() {
    let _sandbox = setup_corpus();
    let v = archive_document_json(json!({
        "document": SAMPLE_DOC,
        "source_type": "summary",
    }))
    .expect("command Result");
    assert_eq!(v.get("ok"), Some(&json!(true)), "archive failed: {v}");
    let id = v["id"].as_str().expect("id");
    assert_eq!(id.len(), 32);
    assert!(v.get("common_path").and_then(|c| c.as_str()).is_some());
    assert!(v.get("error").is_none());
}

#[test]
fn archive_document_json_propagates_status_errors() {
    let _sandbox = setup_corpus();
    let v = archive_document_json(json!({ "document": "", "source_type": "note" }))
        .expect("business errors return Ok(Value)");
    assert!(v.get("error").is_some(), "expected error payload: {v}");
    let status = v.get("_status").and_then(|s| s.as_u64()).unwrap_or(0);
    assert!(status >= 400, "expected _status>=400, got {v}");
}

#[test]
fn archive_document_json_conflict_returns_409() {
    let _sandbox = setup_corpus();
    let payload = json!({ "document": SAMPLE_DOC });
    let first = archive_document_json(payload.clone()).expect("first");
    assert_eq!(first.get("ok"), Some(&json!(true)));
    let second = archive_document_json(payload).expect("second");
    assert!(second.get("error").is_some(), "expected conflict: {second}");
    assert_eq!(second.get("_status"), Some(&json!(409)));
}
