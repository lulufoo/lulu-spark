//! Host unstage_chat_staged: remove one Stage F-handle; reject in-flight; not an agent tool.

use std::sync::Arc;

use crate::commands::ai_assistant::{create_chat_session_json, unstage_chat_staged_json};
use crate::agent::diagnostics::TraceId;
use crate::agent::tools::{host, stage};
use crate::agent::progress::ProgressSink;
use crate::agent::r#loop;
use crate::agent::session::{self, StagedEntry};
use crate::test_support::TestSandbox;

fn with_cmd_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    f();
}

fn seed_staged(sid: &str, entries: &[(&str, &str, &str)]) {
    let mut sess = session::load_session(sid).expect("load");
    sess.staged = entries
        .iter()
        .map(|(id, path, title)| StagedEntry {
            id: (*id).into(),
            path: (*path).into(),
            title: (*title).into(),
            source: None,
        })
        .collect();
    session::save_session(&sess).expect("save");
}

#[test]
fn unstage_removes_one_entry_without_renumbering() {
    with_cmd_sandbox(|| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        seed_staged(
            &sid,
            &[
                ("F1", "/tmp/a.md", "A"),
                ("F2", "/tmp/b.md", "B"),
            ],
        );
        let out = unstage_chat_staged_json(&sid, "F1").expect("unstage");
        assert_eq!(out["session_id"], sid);
        let staged = out["staged"].as_array().expect("staged");
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0]["id"], "F2");
        assert_eq!(staged[0]["title"], "B");
        let loaded = session::load_session(&sid).expect("reload");
        assert_eq!(loaded.staged.len(), 1);
        assert_eq!(loaded.staged[0].id, "F2");
    });
}

#[test]
fn unstage_missing_id_errors_without_write() {
    with_cmd_sandbox(|| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        seed_staged(&sid, &[("F1", "/tmp/a.md", "A")]);
        let err = unstage_chat_staged_json(&sid, "F9").expect_err("missing");
        assert!(err.contains("not found"), "got {err}");
        assert_eq!(session::load_session(&sid).expect("load").staged.len(), 1);
    });
}

#[test]
fn unstage_unknown_session_errors() {
    with_cmd_sandbox(|| {
        let err = unstage_chat_staged_json("sess_missing", "F1").expect_err("unknown");
        assert!(!err.is_empty(), "got {err}");
    });
}

#[test]
fn unstage_in_flight_is_rejected() {
    with_cmd_sandbox(|| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        seed_staged(&sid, &[("F1", "/tmp/a.md", "A")]);
        let sink: ProgressSink = Arc::new(|_desc| {});
        r#loop::try_begin_chat_turn(&sid, &TraceId::from_optional(None), sink).expect("begin");
        let err = unstage_chat_staged_json(&sid, "F1").expect_err("in flight");
        assert!(err.contains("running"), "got {err}");
        assert_eq!(session::load_session(&sid).expect("load").staged.len(), 1);
        r#loop::end_chat_turn(&sid);
        unstage_chat_staged_json(&sid, "F1").expect("after flight");
        assert!(session::load_session(&sid).expect("load").staged.is_empty());
    });
}

#[test]
fn host_and_stage_catalogs_have_no_unstage() {
    for catalog in [host::catalog(), stage::catalog()] {
        for name in ["unstage", "unstage_chat_staged", "delete_staged", "remove_staged"] {
            assert!(!catalog.contains(name), "{name} must not be an agent tool");
            assert!(!host::is_builtin(name) && !stage::is_builtin(name), "{name}");
        }
        let blob = format!("{:?}", catalog.definitions);
        assert!(!blob.contains("unstage"), "tool defs must not mention unstage: {blob}");
    }
}
