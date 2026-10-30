use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::services::docs::{create_doc, update_doc_content};
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30

---

摘要正文，enough content.
"#;

const PNG: &[u8] = b"\x89PNG\r\n";

fn setup_notes() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    sandbox
}

fn stage_bytes(sandbox: &TestSandbox, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join(name);
    fs::write(&p, bytes).expect("write");
    p.canonicalize().expect("canon")
}

fn theme_dir() -> PathBuf {
    crate::test_support::default_workspace_dir()
        .join("raw")
        .join(crate::test_support::default_inbox_id())
        .join("notes")
}

#[test]
fn update_doc_keeps_orphan_files_and_omits_them_from_the_result() {
    let sandbox = setup_notes();
    let a = stage_bytes(&sandbox, "keep.png", PNG);
    let b = stage_bytes(&sandbox, "gone.png", PNG);
    let source = stage_bytes(
        &sandbox,
        "source.md",
        format!("{SAMPLE_DOC}\n\n![a](keep.png)\n![b](gone.png)\n").as_bytes(),
    );
    let created = create_doc(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": [a.to_str().unwrap(), b.to_str().unwrap()],
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let attached = theme_dir().join(".attached").join(id);
    let updated = update_doc_content(&json!({
            "id": id,
            "content": format!("{SAMPLE_DOC}\n\n![a](.attached/{id}/keep.png)\n"),
            "digest": "never",
        }),
    );
    assert_eq!(updated.get("ok"), Some(&json!(true)), "{updated}");
    assert!(updated.get("unmatched_image_refs").is_none());
    assert!(updated.get("orphan").is_none());
    assert!(attached.join("keep.png").is_file());
    assert!(attached.join("gone.png").is_file());
}
