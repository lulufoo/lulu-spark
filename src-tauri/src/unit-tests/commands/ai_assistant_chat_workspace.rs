//! Host list_chat_workspace: session scratch files; not an agent tool.

use std::fs;

use crate::agent::r#loop;
use crate::agent::tools::{host, stage};
use crate::commands::ai_assistant::{
    create_chat_session_json, delete_chat_workspace_file_json, list_chat_workspace_json,
};
use crate::agent::diagnostics::TraceId;
use crate::agent::progress::ProgressSink;
use std::sync::Arc;
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
        for name in ["list_chat_workspace", "delete_chat_workspace_file"] {
            assert!(!catalog.contains(name));
            assert!(!host::is_builtin(name));
            assert!(!stage::is_builtin(name));
        }
    }
}

#[test]
fn delete_chat_workspace_file_removes_session_file() {
    with_bound_sandbox(|sandbox| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let root = sandbox.cache_dir().join("agent-workspace").join(&sid);
        fs::create_dir_all(&root).expect("dir");
        let path = root.join("pad.md");
        fs::write(&path, "scratch").expect("write");
        let out = delete_chat_workspace_file_json(&sid, &path.to_string_lossy()).expect("delete");
        assert_eq!(out["files"].as_array().expect("files").len(), 0);
        assert!(!path.exists());
    });
}

#[test]
fn delete_chat_workspace_file_rejects_outside_and_in_flight() {
    with_bound_sandbox(|sandbox| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let outside = sandbox.data_dir().join("notes").join("keep.md");
        fs::create_dir_all(outside.parent().unwrap()).expect("notes");
        fs::write(&outside, "keep").expect("write");
        let err = delete_chat_workspace_file_json(&sid, &outside.to_string_lossy())
            .expect_err("outside");
        assert!(err.contains("SESSION_WORKSPACE_DIR"), "{err}");
        assert_eq!(fs::read_to_string(&outside).expect("kept"), "keep");

        let root = sandbox.cache_dir().join("agent-workspace").join(&sid);
        fs::create_dir_all(&root).expect("dir");
        let path = root.join("pad.md");
        fs::write(&path, "scratch").expect("write");
        let sink: ProgressSink = Arc::new(|_desc| {});
        r#loop::try_begin_chat_turn(&sid, &TraceId::from_optional(None), sink).expect("begin");
        let busy = delete_chat_workspace_file_json(&sid, &path.to_string_lossy())
            .expect_err("in flight");
        assert!(busy.contains("running"), "{busy}");
        assert!(path.exists());
        r#loop::end_chat_turn(&sid);
    });
}
