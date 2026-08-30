//! Host delete_chat_session: wipe storage, reject in-flight, clear live id.

use std::sync::Arc;

use crate::commands::ai_assistant::{
    create_chat_session_json, delete_chat_session_json, list_chat_sessions_json,
};
use crate::services::agent::diagnostics::TraceId;
use crate::services::agent::progress::ProgressSink;
use crate::services::agent::r#loop;
use crate::services::agent::session;
use crate::test_support::TestSandbox;

fn with_cmd_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    f();
}

#[test]
fn delete_chat_session_removes_file_and_list_entry() {
    with_cmd_sandbox(|| {
        let first = create_chat_session_json().expect("create first");
        let first_id = first["session_id"].as_str().expect("first id").to_string();
        let second = create_chat_session_json().expect("create second");
        let second_id = second["session_id"].as_str().expect("second id").to_string();
        let path = session::session_file_path(&first_id).expect("path");
        assert!(path.is_file());

        let listed = delete_chat_session_json(&first_id).expect("delete");
        let sessions = listed["sessions"].as_array().expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["session_id"], second_id);
        assert!(!path.is_file(), "session file must be gone");
        assert_eq!(listed["current_session_id"], second_id);
    });
}

#[test]
fn delete_current_chat_session_clears_live_id() {
    with_cmd_sandbox(|| {
        let created = create_chat_session_json().expect("create");
        let id = created["session_id"].as_str().expect("id").to_string();
        let listed = delete_chat_session_json(&id).expect("delete current");
        assert!(listed["current_session_id"].as_str().unwrap_or("").is_empty());
        assert_eq!(listed["sessions"].as_array().map(|a| a.len()), Some(0));
        assert!(list_chat_sessions_json().expect("relist")["sessions"]
            .as_array()
            .is_some_and(|a| a.is_empty()));
    });
}

#[test]
fn delete_unknown_chat_session_errors() {
    with_cmd_sandbox(|| {
        let err = delete_chat_session_json("sess_missing").expect_err("unknown");
        assert!(err.contains("Session not found"), "got {err}");
    });
}

#[test]
fn delete_in_flight_chat_session_is_rejected() {
    with_cmd_sandbox(|| {
        let created = create_chat_session_json().expect("create");
        let id = created["session_id"].as_str().expect("id").to_string();
        let sink: ProgressSink = Arc::new(|_desc| {});
        r#loop::try_begin_chat_turn(&id, &TraceId::from_optional(None), sink).expect("begin");
        let err = delete_chat_session_json(&id).expect_err("in flight");
        assert!(err.contains("running"), "got {err}");
        assert!(session::session_file_path(&id).expect("path").is_file());
        r#loop::end_chat_turn(&id);
        delete_chat_session_json(&id).expect("delete after flight");
        assert!(!session::session_file_path(&id).expect("path").is_file());
    });
}
