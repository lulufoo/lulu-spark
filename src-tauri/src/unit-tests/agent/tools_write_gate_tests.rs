use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::agent::approval::{self, ApprovalChoice};
use crate::agent::fence::spark::expand_for_business_key;
use crate::agent::fence::{stored_path, PathFence};
use crate::agent::r#loop;
use crate::agent::session;
use crate::agent::tools::local::{self, Local};
use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    approval::reset_for_tests();
    registry::clear_for_tests();
    registry::seed_defaults();
    f(&sandbox);
}

fn scratch_fence(session_id: &str) -> PathFence {
    expand_for_business_key(SEEDED_BUSINESS_KEY)
        .expect("fence")
        .with_session_scratch(session_id)
        .expect("scratch")
}

fn outside(sandbox: &TestSandbox, name: &str) -> PathBuf {
    sandbox.config_dir().join("external").join(name)
}

fn call_write(session: &mut session::Session, fence: &PathFence, path: &str, content: &str) -> Local {
    local::call(
        "write",
        &json!({ "path": path, "content": content }),
        session,
        Some(fence),
        false,
        Duration::from_secs(3),
        &|| false,
    )
}

fn call_stage(session: &mut session::Session, fence: &PathFence, path: &str) -> Local {
    local::call(
        "stage",
        &json!({ "path": path }),
        session,
        Some(fence),
        false,
        Duration::from_secs(3),
        &|| false,
    )
}

fn resolve_next(choice: ApprovalChoice) {
    for _ in 0..200 {
        if let Some(id) = approval::pending_ids().into_iter().next() {
            approval::resolve(&id, choice);
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("no approval request");
}

fn allow_once_in_background() -> thread::JoinHandle<()> {
    thread::spawn(|| resolve_next(ApprovalChoice::AllowOnce))
}

fn content(local: &Local) -> String {
    match local {
        Local::Done(result) => result.content.clone(),
        Local::Cancelled => "cancelled".into(),
        Local::Stopped(_, result) => result.content.clone(),
    }
}

fn is_error(local: &Local) -> bool {
    matches!(local, Local::Done(result) if result.is_error)
}

#[test]
fn workspace_write_does_not_ask() {
    with_sandbox(|_| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let scratch = fence
            .session_scratch_root(&sess.session_id)
            .expect("root")
            .expect("scratch");
        let pad = scratch.join("pad.md");
        let result = call_write(&mut sess, &fence, pad.to_str().unwrap(), "ok");
        assert!(!is_error(&result), "{}", content(&result));
        assert!(approval::pending_ids().is_empty());
        assert_eq!(fs::read_to_string(&pad).expect("pad"), "ok");
        assert!(sess.staged.is_empty());
    });
}

#[test]
fn staged_rewrite_does_not_ask() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let mut fence = scratch_fence(&sess.session_id);
        let path = outside(sandbox, "already.md");
        fs::create_dir_all(path.parent().unwrap()).expect("dir");
        fs::write(&path, "old").expect("seed");
        sess.register_staged(path.to_str().unwrap(), None)
            .expect("stage");
        fence.grant_staged_file(stored_path(path.clone()));
        let result = call_write(&mut sess, &fence, path.to_str().unwrap(), "new");
        assert!(!is_error(&result), "{}", content(&result));
        assert!(approval::pending_ids().is_empty());
        assert_eq!(fs::read_to_string(&path).expect("file"), "new");
    });
}

#[test]
fn outside_write_waits_then_writes_and_stages() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let path = outside(sandbox, "created.md");
        fs::create_dir_all(path.parent().unwrap()).expect("dir");
        let waiter = allow_once_in_background();
        let result = call_write(&mut sess, &fence, path.to_str().unwrap(), "hello");
        waiter.join().expect("resolver");
        assert!(!is_error(&result), "{}", content(&result));
        assert_eq!(fs::read_to_string(&path).expect("file"), "hello");
        assert_eq!(sess.staged.len(), 1);
        assert_eq!(sess.staged[0].path, stored_path(path).to_string_lossy());
    });
}

#[test]
fn deny_leaves_file_off_disk() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let path = outside(sandbox, "denied.md");
        fs::create_dir_all(path.parent().unwrap()).expect("dir");
        let waiter = thread::spawn(|| resolve_next(ApprovalChoice::Deny));
        let result = call_write(&mut sess, &fence, path.to_str().unwrap(), "nope");
        waiter.join().expect("resolver");
        assert!(is_error(&result));
        assert!(content(&result).contains("approval denied"));
        assert!(!path.exists());
        assert!(sess.staged.is_empty());
    });
}

#[test]
fn stage_missing_path_after_allow() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let path = outside(sandbox, "ghost.md");
        fs::create_dir_all(path.parent().unwrap()).expect("dir");
        let waiter = allow_once_in_background();
        let result = call_stage(&mut sess, &fence, path.to_str().unwrap());
        waiter.join().expect("resolver");
        assert!(!is_error(&result), "{}", content(&result));
        assert!(!path.exists());
        assert_eq!(sess.staged.len(), 1);
    });
}

#[test]
fn copy_outside_dest_uses_same_gate() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let scratch = fence
            .session_scratch_root(&sess.session_id)
            .expect("root")
            .expect("scratch");
        fs::create_dir_all(&scratch).expect("scratch");
        let src = scratch.join("src.md");
        fs::write(&src, "copy-me").expect("src");
        let dest = outside(sandbox, "copied.md");
        fs::create_dir_all(dest.parent().unwrap()).expect("dir");
        let waiter = allow_once_in_background();
        let result = local::call(
            "copy",
            &json!({
                "source_path": src.to_string_lossy(),
                "dest_path": dest.to_string_lossy()
            }),
            &mut sess,
            Some(&fence),
            false,
            Duration::from_secs(3),
            &|| false,
        );
        waiter.join().expect("resolver");
        assert!(!is_error(&result), "{}", content(&result));
        assert_eq!(fs::read_to_string(&dest).expect("dest"), "copy-me");
        assert_eq!(sess.staged.len(), 1);
    });
}

#[test]
fn hard_deny_does_not_ask_or_write() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let note = sandbox.data_dir().join("notes").join("secret.md");
        fs::create_dir_all(note.parent().unwrap()).expect("notes");
        let result = call_write(&mut sess, &fence, note.to_str().unwrap(), "x");
        assert!(is_error(&result));
        assert!(
            content(&result).contains("hard-deny") || content(&result).contains("read fence"),
            "{}",
            content(&result)
        );
        assert!(approval::pending_ids().is_empty());
        assert!(!note.exists() || fs::read_to_string(&note).unwrap_or_default() != "x");
        assert!(sess.staged.is_empty());
    });
}

#[test]
fn stage_tool_asks() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let path = outside(sandbox, "tool.md");
        fs::create_dir_all(path.parent().unwrap()).expect("dir");
        fs::write(&path, "body").expect("file");
        let waiter = allow_once_in_background();
        let result = call_stage(&mut sess, &fence, path.to_str().unwrap());
        waiter.join().expect("resolver");
        assert!(!is_error(&result), "{}", content(&result));
        assert_eq!(sess.staged.len(), 1);
    });
}

#[test]
fn unstage_then_rewrite_asks_again() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let path = outside(sandbox, "again.md");
        fs::create_dir_all(path.parent().unwrap()).expect("dir");
        let first = thread::spawn(|| resolve_next(ApprovalChoice::AllowSession));
        let written = call_write(&mut sess, &fence, path.to_str().unwrap(), "one");
        first.join().expect("first");
        assert!(!is_error(&written), "{}", content(&written));
        let id = sess.staged[0].id.clone();
        session::unstage_entry(&sess.session_id, &id).expect("unstage");
        let mut sess = session::load_session(&sess.session_id).expect("reload");
        let fence = scratch_fence(&sess.session_id);
        let second = thread::spawn(|| resolve_next(ApprovalChoice::Deny));
        let denied = call_write(&mut sess, &fence, path.to_str().unwrap(), "two");
        second.join().expect("second");
        assert!(is_error(&denied));
        assert_eq!(fs::read_to_string(&path).expect("kept"), "one");
    });
}
