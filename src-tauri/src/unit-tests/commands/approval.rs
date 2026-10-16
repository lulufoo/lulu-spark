use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::agent::approval::{create_for, is_pending, reset_for_tests, ApprovalChoice};
use crate::agent::r#loop;
use crate::agent::session;
use crate::commands::approval::{
    approval_request_payload, resolve_approval_json, EVENT_APPROVAL_REQUESTED,
};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    reset_for_tests();
    f();
}

fn wait_until_pending(request_id: &str) {
    for _ in 0..100 {
        if is_pending(request_id) {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("approval {request_id} was not pending");
}

#[test]
fn event_payload_has_request_id_type_path_and_session() {
    assert_eq!(EVENT_APPROVAL_REQUESTED, "ai-assistant:approval-requested");
    let payload = approval_request_payload("req-1", "/tmp/doc.md", "spark_chat_abc");
    let value = serde_json::to_value(&payload).expect("json");
    assert_eq!(value["request_id"], "req-1");
    assert_eq!(value["type"], "fs");
    assert_eq!(value["path"], "/tmp/doc.md");
    assert_eq!(value["session_id"], "spark_chat_abc");
}

#[test]
fn invoke_same_id_finishes_wait() {
    with_sandbox(|| {
        let sess = session::create_session().expect("session");
        let id = format!("{}-ipc", sess.session_id);
        let session_id = sess.session_id.clone();
        let waiter = thread::spawn(move || create_for(&id, &session_id, Duration::from_secs(2)));
        let pending_id = format!("{}-ipc", sess.session_id);
        wait_until_pending(&pending_id);
        let out = resolve_approval_json(&pending_id, "deny").expect("resolve");
        assert_eq!(out["ok"], true);
        assert_eq!(out["choice"], "deny");
        assert_eq!(waiter.join().expect("waiter"), ApprovalChoice::Deny);
    });
}

#[test]
fn unknown_id_returns_error() {
    with_sandbox(|| {
        let err = resolve_approval_json("missing-id", "allow_once").expect_err("unknown");
        assert!(err.contains("not found"), "{err}");
    });
}

#[test]
fn duplicate_reply_does_not_change_decided_result() {
    with_sandbox(|| {
        let sess = session::create_session().expect("session");
        let id = format!("{}-dup", sess.session_id);
        let session_id = sess.session_id.clone();
        let waiter = thread::spawn({
            let id = id.clone();
            move || create_for(&id, &session_id, Duration::from_secs(2))
        });
        wait_until_pending(&id);
        let first = resolve_approval_json(&id, "allow_once").expect("first");
        assert_eq!(first["choice"], "allow_once");
        let err = resolve_approval_json(&id, "allow_session").expect_err("duplicate");
        assert!(err.contains("not found"), "{err}");
        assert_eq!(waiter.join().expect("waiter"), ApprovalChoice::AllowOnce);
    });
}

#[test]
fn handler_and_acl_register_resolve_approval() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib = fs::read_to_string(root.join("src/lib.rs")).expect("lib");
    assert!(
        lib.contains("commands::approval::resolve_approval"),
        "generate_handler must register resolve_approval"
    );
    let toml = fs::read_to_string(root.join("permissions/write-api.toml")).expect("toml");
    assert!(
        toml.contains("\"resolve_approval\""),
        "write-api.toml must allow resolve_approval"
    );
    let boot = fs::read_to_string(root.join("../frontend/src/boot.ts")).expect("boot");
    let hub = fs::read_to_string(root.join("../frontend/src/home/commands/approval.ts"))
        .expect("hub");
    assert!(boot.contains("startApprovalHub"), "{boot} must start listen");
    assert!(hub.contains("ai-assistant:approval-requested"));
}
