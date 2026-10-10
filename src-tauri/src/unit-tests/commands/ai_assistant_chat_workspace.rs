//! Host list_chat_workspace: session scratch files; not an agent tool.

use std::fs;

use crate::agent::r#loop;
use crate::agent::tools::{host, stage};
use crate::commands::ai_assistant::{create_chat_session_json, list_chat_workspace_json};
use crate::mcp_host::registry::SEEDED_BUSINESS_KEY;
use crate::test_support::TestSandbox;
use serde_json::json;

fn with_bound_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    crate::mcp_host::registry::seed_defaults();
    r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("bind");
    f(&sandbox);
}

#[test]
fn list_chat_workspace_rejects_missing_session() {
    with_bound_sandbox(|_| {
        let err = list_chat_workspace_json("").expect_err("empty");
        assert!(err.contains("session_id"), "{err}");
        let missing = list_chat_workspace_json("spark_chat_missing").expect_err("unknown");
        assert!(!missing.is_empty(), "{missing}");
    });
}

#[test]
fn list_chat_workspace_empty_when_scratch_missing() {
    with_bound_sandbox(|_| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let out = list_chat_workspace_json(&sid).expect("list");
        assert_eq!(out["session_id"], sid);
        assert_eq!(out["files"].as_array().expect("files").len(), 0);
    });
}

#[test]
fn list_chat_workspace_lists_session_files() {
    with_bound_sandbox(|sandbox| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let root = sandbox.cache_dir().join("agent-workspace").join(&sid);
        fs::create_dir_all(root.join("nested")).expect("dir");
        fs::write(root.join("top.md"), "top").expect("top");
        fs::write(root.join("nested").join("inner.md"), "inner").expect("inner");
        let out = list_chat_workspace_json(&sid).expect("list");
        let files = out["files"].as_array().expect("files");
        let titles: Vec<_> = files
            .iter()
            .map(|row| row["title"].as_str().unwrap_or(""))
            .collect();
        assert_eq!(titles, vec!["nested/inner.md", "top.md"]);
    });
}

#[test]
fn host_catalog_has_no_list_chat_workspace() {
    for catalog in [host::catalog(), stage::catalog()] {
        assert!(!catalog.contains("list_chat_workspace"));
        assert!(!host::is_builtin("list_chat_workspace"));
        assert!(!stage::is_builtin("list_chat_workspace"));
    }
}
