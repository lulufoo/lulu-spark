use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use crate::agent::approval::{
    cancel, create_for, is_pending, reset_for_tests, resolve, ApprovalChoice, APPROVAL_TIMEOUT,
};
use crate::agent::fence::spark::expand_for_business_key;
use crate::agent::permission::{
    decide, load_records, PermissionDecision, PermissionKind, PermissionRequest,
};
use crate::agent::r#loop;
use crate::agent::session;
use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    reset_for_tests();
    registry::clear_for_tests();
    registry::seed_defaults();
    f(&sandbox);
}

fn outside_file(sandbox: &TestSandbox, name: &str) -> PathBuf {
    let path = sandbox.config_dir().join("external").join(name);
    fs::create_dir_all(path.parent().unwrap()).expect("external");
    fs::write(&path, "body").expect("write");
    path
}

fn wait_create(request_id: String, session_id: String) -> thread::JoinHandle<ApprovalChoice> {
    thread::spawn(move || create_for(&request_id, &session_id, Duration::from_secs(2)))
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

fn settle(handle: thread::JoinHandle<ApprovalChoice>) -> ApprovalChoice {
    handle.join().expect("waiter")
}

#[test]
fn timeout_constant_is_five_minutes() {
    assert_eq!(APPROVAL_TIMEOUT, Duration::from_secs(300));
}

#[test]
fn resolve_allow_once_does_not_write_session_cache() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        let id = format!("{}-once", sess.session_id);
        let waiter = wait_create(id.clone(), sess.session_id.clone());
        wait_until_pending(&id);
        assert_eq!(resolve(&id, ApprovalChoice::AllowOnce), ApprovalChoice::AllowOnce);
        assert_eq!(settle(waiter), ApprovalChoice::AllowOnce);

        let db = session::session_file_path(&sess.session_id).expect("db");
        let records = load_records(&db).unwrap_or_default();
        assert!(!records.iter().any(|r| r.scope
            == crate::agent::permission::PermissionScope::Session
            && r.resource.is_none()));

        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY)
            .expect("fence")
            .with_session_scratch(&sess.session_id)
            .expect("scratch");
        let path = outside_file(sandbox, "once.md");
        let decision = decide(
            &PermissionRequest {
                kind: PermissionKind::Fs,
                path,
                session_id: sess.session_id.clone(),
                request_id: id,
            },
            &fence,
            &records,
        );
        assert_eq!(decision, PermissionDecision::Ask);
    });
}

#[test]
fn resolve_allow_session_then_permission_allows() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        let id = format!("{}-session", sess.session_id);
        let waiter = wait_create(id.clone(), sess.session_id.clone());
        wait_until_pending(&id);
        assert_eq!(
            resolve(&id, ApprovalChoice::AllowSession),
            ApprovalChoice::AllowSession
        );
        assert_eq!(settle(waiter), ApprovalChoice::AllowSession);

        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY)
            .expect("fence")
            .with_session_scratch(&sess.session_id)
            .expect("scratch");
        let path = outside_file(sandbox, "after-session.md");
        let db = session::session_file_path(&sess.session_id).expect("db");
        let records = load_records(&db).expect("records");
        assert_eq!(
            decide(
                &PermissionRequest {
                    kind: PermissionKind::Fs,
                    path,
                    session_id: sess.session_id,
                    request_id: id,
                },
                &fence,
                &records,
            ),
            PermissionDecision::Allow
        );
    });
}

#[test]
fn resolve_deny_returns_deny() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("session");
        let id = format!("{}-deny", sess.session_id);
        let waiter = wait_create(id.clone(), sess.session_id.clone());
        wait_until_pending(&id);
        assert_eq!(resolve(&id, ApprovalChoice::Deny), ApprovalChoice::Deny);
        assert_eq!(settle(waiter), ApprovalChoice::Deny);
    });
}

#[test]
fn timeout_is_deny() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("session");
        let id = format!("{}-timeout", sess.session_id);
        let session_id = sess.session_id.clone();
        let choice = create_for(&id, &session_id, Duration::from_millis(30));
        assert_eq!(choice, ApprovalChoice::Deny);
        assert_eq!(resolve(&id, ApprovalChoice::AllowSession), ApprovalChoice::Deny);
    });
}

#[test]
fn cancel_is_deny() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("session");
        let id = format!("{}-cancel", sess.session_id);
        let waiter = wait_create(id.clone(), sess.session_id.clone());
        wait_until_pending(&id);
        assert_eq!(cancel(&id), ApprovalChoice::Deny);
        assert_eq!(settle(waiter), ApprovalChoice::Deny);
    });
}

#[test]
fn unknown_id_is_deny_and_does_not_resolve_other_request() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("session");
        let live = format!("{}-live", sess.session_id);
        let waiter = wait_create(live.clone(), sess.session_id.clone());
        wait_until_pending(&live);
        assert_eq!(
            resolve("missing-id", ApprovalChoice::AllowSession),
            ApprovalChoice::Deny
        );
        assert_eq!(resolve(&live, ApprovalChoice::Deny), ApprovalChoice::Deny);
        assert_eq!(settle(waiter), ApprovalChoice::Deny);
    });
}
