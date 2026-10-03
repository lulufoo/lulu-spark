use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::commands::write::create_note_json;
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30
> 来源：theme-summary
> 导航：[digest](../../../digest/inbox/test-topic/202606191430-test-slug.md)

---

摘要正文，enough content.
"#;

fn setup_notes() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = sandbox.spark_root().join("notes");
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("index");
    sandbox
}

fn stage_source(sandbox: &TestSandbox, content: &str) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join("source.md");
    fs::write(&p, content).expect("write");
    p.canonicalize().expect("canon")
}

#[test]
fn create_note_json_success_returns_entry_handle() {
    let sandbox = setup_notes();
    let path = stage_source(&sandbox, SAMPLE_DOC);
    let v = create_note_json(json!({
        "source_path": path.to_str().unwrap(),
        "title": "Test Title",
        "source_type": "summary",
        "digest": "never",
    }))
    .expect("command Result");
    assert_eq!(v.get("ok"), Some(&json!(true)), "archive failed: {v}");
    let id = v["id"].as_str().expect("id");
    assert_eq!(id.len(), 32);
    assert!(v.get("common_path").and_then(|c| c.as_str()).is_some());
    assert!(v.get("error").is_none());
}

#[test]
fn create_note_json_propagates_status_errors() {
    let _sandbox = setup_notes();
    let v = create_note_json(json!({ "source_type": "jot" }))
        .expect("business errors return Ok(Value)");
    assert!(v.get("error").is_some(), "expected error payload: {v}");
    let status = v.get("_status").and_then(|s| s.as_u64()).unwrap_or(0);
    assert!(status >= 400, "expected _status>=400, got {v}");
}

#[test]
fn create_note_json_rejects_document_field() {
    let sandbox = setup_notes();
    let path = stage_source(&sandbox, SAMPLE_DOC);
    let v = create_note_json(json!({
        "document": SAMPLE_DOC,
        "source_path": path.to_str().unwrap(),
    }))
    .expect("Ok(Value)");
    assert_eq!(v.get("_status"), Some(&json!(400)));
    assert!(
        v["error"]
            .as_str()
            .unwrap_or("")
            .contains("source_path"),
        "{v}"
    );
}

#[test]
fn create_note_json_second_write_gets_new_filename() {
    let sandbox = setup_notes();
    let path = stage_source(&sandbox, SAMPLE_DOC);
    let payload = json!({
        "source_path": path.to_str().unwrap(),
        "title": "Test Title",
        "source_type": "summary",
        "digest": "never",
    });
    let first = create_note_json(payload.clone()).expect("first");
    let second = create_note_json(payload).expect("second");
    assert_eq!(first.get("ok"), Some(&json!(true)));
    assert_eq!(second.get("ok"), Some(&json!(true)));
    assert_ne!(first["common_path"], second["common_path"]);
}
