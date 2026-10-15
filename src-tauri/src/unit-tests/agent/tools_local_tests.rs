use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::agent::fence::PathFence;
use crate::agent::prompt::{read_tool_timeout_text, write_tool_timeout_text};
use crate::agent::r#loop;
use crate::agent::session::{self, Session};
use crate::agent::tools::host;
use crate::test_support::TestSandbox;

use super::{call, settle, timeout_result, Local, Waited};

const LIMIT: Duration = Duration::from_secs(5);

fn never_cancelled() -> bool {
    false
}

fn unique_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "wb-local-tools-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos(),
        label
    ));
    fs::create_dir_all(&dir).expect("create unique dir");
    dir
}

fn read_fence(root: &std::path::Path) -> PathFence {
    PathFence {
        read_allow: vec![root.to_path_buf()],
        read_deny: vec![],
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: None,
    }
}

fn with_live_chat<F: FnOnce(&mut Session)>(f: F) {
    let _sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    let sid = r#loop::create_chat_session_core().expect("chat")["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    let mut sess = session::load_session(&sid).expect("load");
    f(&mut sess);
}

fn done(local: Local) -> crate::agent::tools::ToolResult {
    match local {
        Local::Done(result) => result,
        _ => panic!("expected the tool to finish"),
    }
}

#[test]
fn a_read_only_timeout_is_a_tool_error_with_the_retry_text() {
    let result = timeout_result(true);
    assert!(result.is_error);
    assert_eq!(result.content, read_tool_timeout_text());
}

#[test]
fn a_timeout_of_a_tool_that_changes_things_says_the_result_is_unknown() {
    let result = timeout_result(false);
    assert!(result.is_error);
    assert_eq!(result.content, write_tool_timeout_text());
}

#[test]
fn settle_passes_a_finished_value_through() {
    assert!(matches!(settle(Waited::Done(5), true), Ok(5)));
}

#[test]
fn settle_turns_a_timeout_into_the_matching_text() {
    for (read_only, text) in [
        (true, read_tool_timeout_text()),
        (false, write_tool_timeout_text()),
    ] {
        match settle(Waited::<()>::TimedOut, read_only) {
            Err(Local::TimedOut(result)) => assert_eq!(result.content, text),
            _ => panic!("expected a timeout"),
        }
    }
}

#[test]
fn settle_keeps_cancel_apart_from_a_tool_error() {
    assert!(matches!(
        settle(Waited::<()>::Cancelled, true),
        Err(Local::Cancelled)
    ));
}

#[test]
fn settle_reports_a_crash_as_a_tool_error() {
    match settle(Waited::<()>::Crashed, true) {
        Err(Local::Done(result)) => assert!(result.is_error),
        _ => panic!("expected a tool error"),
    }
}

#[test]
fn grep_that_runs_out_of_time_fails_instead_of_returning_a_partial_list() {
    let root = unique_dir("grep-late");
    fs::write(root.join("a.txt"), "find-me\n").expect("write");
    let fence = read_fence(&root);
    let args = json!({ "pattern": "find-me", "path": root.to_string_lossy() });

    let late = Instant::now() - Duration::from_secs(1);
    let result = host::call_until("grep", &args, &fence, Some(late));
    assert!(result.is_error);
    assert_eq!(result.content, read_tool_timeout_text());

    let in_time = Instant::now() + Duration::from_secs(60);
    let result = host::call_until("grep", &args, &fence, Some(in_time));
    assert!(!result.is_error, "{}", result.content);
    assert!(result.content.contains("find-me"));
}

#[test]
fn a_file_tool_that_finishes_in_time_returns_its_own_result() {
    with_live_chat(|sess| {
        let root = unique_dir("read-ok");
        fs::write(root.join("a.txt"), "alpha\n").expect("write");
        let fence = read_fence(&root);
        let args = json!({ "path": root.join("a.txt").to_string_lossy() });
        let result = done(call("read", &args, sess, Some(&fence), true, LIMIT, &never_cancelled));
        assert!(!result.is_error, "{}", result.content);
        let body: Value = serde_json::from_str(&result.content).expect("json");
        assert_eq!(body["content"], "1:alpha");
    });
}

#[test]
fn a_file_tool_without_a_fence_fails_without_running() {
    with_live_chat(|sess| {
        let args = json!({ "path": "/tmp/x" });
        let result = done(call("read", &args, sess, None, true, LIMIT, &never_cancelled));
        assert!(result.is_error);
        assert!(result.content.contains("no path fence"));
    });
}

#[test]
fn stage_checks_the_file_in_the_shell_and_registers_it_inline() {
    with_live_chat(|sess| {
        let root = unique_dir("stage-ok");
        let file = root.join("note.md");
        fs::write(&file, "body").expect("write");
        let fence = read_fence(&root);
        let args = json!({ "path": file.to_string_lossy() });
        let result = done(call("stage", &args, sess, Some(&fence), false, LIMIT, &never_cancelled));
        assert!(!result.is_error, "{}", result.content);
        assert_eq!(sess.staged.len(), 1);
        assert_eq!(sess.staged[0].id, "F1");
    });
}

#[test]
fn stage_of_a_missing_file_registers_nothing() {
    with_live_chat(|sess| {
        let root = unique_dir("stage-missing");
        let fence = read_fence(&root);
        let args = json!({ "path": root.join("nope.md").to_string_lossy() });
        let result = done(call("stage", &args, sess, Some(&fence), false, LIMIT, &never_cancelled));
        assert!(result.is_error);
        assert!(sess.staged.is_empty());
    });
}

#[test]
fn list_staged_and_get_staged_run_inline() {
    with_live_chat(|sess| {
        // A time limit of zero would time out any tool that goes through the shell.
        let zero = Duration::ZERO;
        let listed = done(call("list_staged", &json!({}), sess, None, true, zero, &never_cancelled));
        assert!(!listed.is_error, "{}", listed.content);
        assert_eq!(listed.content, "[]");
        let got = done(call("get_staged", &json!({ "id": "F9" }), sess, None, true, zero, &never_cancelled));
        assert!(got.is_error);
        assert!(got.content.contains("staged file not found"));
    });
}
