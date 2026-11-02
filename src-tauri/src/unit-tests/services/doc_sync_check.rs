use serde_json::json;

use super::*;
use crate::services::cloud_sync::CloudSyncError;
use crate::services::doc_sync::tests::{id, Call, FakeCloud};
use crate::services::docs::create_doc_content;
use crate::test_support::TestSandbox;

fn new_doc(title: &str) -> String {
    let created = create_doc_content(&json!({
        "workspace_id": crate::test_support::default_workspace().id,
        "title": title,
        "content": format!("# {title}\n\n这是一段足够长的正文。\n"),
        "digest": "never",
        "source_type": "summary",
    }));
    created["id"].as_str().unwrap_or_else(|| panic!("create failed: {created}")).to_string()
}

#[test]
fn the_check_counts_what_a_run_would_send_and_bring_back() {
    let _sandbox = TestSandbox::new();
    new_doc("Only here");
    let same = new_doc("Same");
    let cloud = FakeCloud::new().with(&same, 0).with(&id(7), 3);

    let answer = check_with(&cloud);

    assert_eq!(answer, json!({ "ok": true, "uploads": 1, "pulls": 1 }));
}

#[test]
fn nothing_to_do_counts_zero() {
    let _sandbox = TestSandbox::new();

    assert_eq!(check_with(&FakeCloud::new()), json!({ "ok": true, "uploads": 0, "pulls": 0 }));
}

#[test]
fn the_check_only_lists_the_cloud_and_sends_nothing() {
    let _sandbox = TestSandbox::new();
    let doc = new_doc("Only here");
    let cloud = FakeCloud::new();

    check_with(&cloud);

    assert_eq!(cloud.calls(), vec![Call::List]);
    assert_eq!(cloud.stored(&doc), None, "a check never sends a document");
}

#[test]
fn a_check_that_cannot_be_made_answers_like_a_run_that_cannot_be_made() {
    let _sandbox = TestSandbox::new();

    for (error, status) in [(CloudSyncError::NotSignedIn, 401), (CloudSyncError::Network("down".into()), 502)] {
        let answer = check_with(&FakeCloud::new().failing_list(error));

        assert_eq!(answer["_status"], json!(status), "{answer}");
        assert!(answer["error"].as_str().is_some_and(|text| text.starts_with("Cannot sync")), "{answer}");
        assert!(answer.get("ok").is_none(), "{answer}");
    }
}
