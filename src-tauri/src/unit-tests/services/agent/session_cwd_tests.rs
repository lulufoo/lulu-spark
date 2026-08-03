//! T5: per-session independent cwd lifecycle + Cursor adapter wiring.

use std::fs;
use std::path::Path;

use serde_json::json;

use crate::services::agent::cursor_adapter::{self, AgentCreateParams, CursorAdapterError};
use crate::services::agent::engine_router::TurnInput;
use crate::services::agent::r#loop;
use crate::services::agent::session_cwd::{self, CleanupStrategy, CwdError};
use crate::services::mcp_server_registry::{self, SEEDED_BUSINESS_KEY};
use crate::services::todo_task;
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();
    r#loop::reset_runtime_for_tests();
    session_cwd::reset_for_tests();
    f();
    session_cwd::reset_for_tests();
    r#loop::reset_runtime_for_tests();
    mcp_server_registry::clear_for_tests();
}

fn key_only_payload(key: &str) -> serde_json::Value {
    json!({ "key": key })
}

/// Capturing double for Cursor Agent SDK Local port.
#[derive(Default)]
struct CapturingSdk {
    creates: Vec<AgentCreateParams>,
    run_count: usize,
}

impl cursor_adapter::CursorAgentSdk for CapturingSdk {
    fn create(&mut self, params: AgentCreateParams) -> Result<(), CursorAdapterError> {
        self.creates.push(params);
        Ok(())
    }

    fn run_turn(&mut self, _message: &str) -> Result<String, CursorAdapterError> {
        self.run_count += 1;
        Ok("cursor-sdk-ok".into())
    }
}

fn assert_dir_exists(path: &Path) {
    assert!(
        path.is_dir(),
        "expected directory at {path:?}, exists={}, is_file={}",
        path.exists(),
        path.is_file()
    );
}

#[test]
fn t5_a1_cwd_mcpservers_combo_is_explicitly_confirmed() {
    // A1 (L3 Must Close Before P4): local.cwd + mcpServers combination.
    assert!(
        session_cwd::CURSOR_LOCAL_A1_CWD_MCPSERVERS_COMBO,
        "A1 must be explicit: per-session local.cwd combined with injected mcpServers"
    );
    assert!(
        cursor_adapter::CURSOR_LOCAL_A1_CWD_MCPSERVERS_COMBO,
        "A1 must also be visible on cursor_adapter delivery surface"
    );
}

#[test]
fn t5_create_session_cwd_makes_independent_observable_directory() {
    with_sandbox(|| {
        let path = session_cwd::create_session_cwd("sess_a").expect("create cwd");
        assert_dir_exists(&path);
        assert!(
            path.to_string_lossy().contains("sess_a"),
            "cwd path should be keyed by session_id: {path:?}"
        );
        let observed = session_cwd::session_cwd_for("sess_a").expect("observable");
        assert_eq!(observed, path);
    });
}

#[test]
fn t5_two_sessions_get_distinct_cwds_not_shared() {
    with_sandbox(|| {
        let a = session_cwd::create_session_cwd("sess_one").expect("a");
        let b = session_cwd::create_session_cwd("sess_two").expect("b");
        assert_ne!(a, b, "sessions must not share cwd");
        assert_dir_exists(&a);
        assert_dir_exists(&b);
        // Marker files prove isolation (writes do not collide).
        fs::write(a.join("marker.txt"), "one").expect("write a");
        fs::write(b.join("marker.txt"), "two").expect("write b");
        assert_eq!(fs::read_to_string(a.join("marker.txt")).unwrap(), "one");
        assert_eq!(fs::read_to_string(b.join("marker.txt")).unwrap(), "two");
    });
}

#[test]
fn t5_cleanup_session_cwd_removes_directory_immediate_strategy() {
    with_sandbox(|| {
        let path = session_cwd::create_session_cwd("sess_clean").expect("create");
        assert_dir_exists(&path);
        session_cwd::cleanup_session_cwd("sess_clean").expect("cleanup");
        assert!(
            !path.exists(),
            "immediate cleanup must remove cwd directory: {path:?}"
        );
        assert!(
            session_cwd::session_cwd_for("sess_clean").is_none(),
            "allocation must be cleared after cleanup"
        );
        assert_eq!(
            session_cwd::DEFAULT_CLEANUP_STRATEGY,
            CleanupStrategy::ImmediateDelete,
            "default cleanup strategy must be explicit"
        );
    });
}

#[test]
fn t5_cleanup_failure_retries_then_retains_with_explicit_error() {
    with_sandbox(|| {
        let path = session_cwd::create_session_cwd("sess_sticky").expect("create");
        // Force cleanup failure via test hook — must not silently ignore.
        session_cwd::force_cleanup_fail_for_tests(true);
        let err = session_cwd::cleanup_session_cwd("sess_sticky")
            .expect_err("cleanup failure must not be silent");
        match err {
            CwdError::CleanupRetained(msg) => {
                assert!(
                    !msg.is_empty(),
                    "retained cleanup must carry a warning message"
                );
            }
            other => panic!("expected CleanupRetained, got {other:?}"),
        }
        // Directory retained (not silently ignored / not pretended cleaned).
        assert!(
            path.exists(),
            "on cleanup failure after retry, directory must be retained"
        );
        session_cwd::force_cleanup_fail_for_tests(false);
    });
}

#[test]
fn t5_create_failure_is_explicit_never_shares_cwd() {
    with_sandbox(|| {
        session_cwd::force_create_fail_for_tests(true);
        let err = session_cwd::create_session_cwd("sess_fail_create")
            .expect_err("create failure must surface");
        assert!(
            matches!(err, CwdError::CreateFailed(_)),
            "expected CreateFailed, got {err:?}"
        );
        assert!(
            session_cwd::session_cwd_for("sess_fail_create").is_none(),
            "failed create must not register a shared/global cwd"
        );
        // Alternate isolation was attempted (still per-session) then exhausted —
        // never silent shared-cwd downgrade.
        assert!(
            session_cwd::alternate_isolation_attempted_for_tests(),
            "create failure must try alternate isolation strategy before giving up"
        );
        session_cwd::force_create_fail_for_tests(false);
    });
}

#[test]
fn t5_invalid_session_id_rejects_without_creating_shared_cwd() {
    with_sandbox(|| {
        for bad in ["", "  ", "../escape", "a/b", "a\\b"] {
            let err = session_cwd::create_session_cwd(bad).expect_err("invalid id");
            assert!(
                matches!(err, CwdError::InvalidSessionId),
                "bad id {bad:?} → InvalidSessionId, got {err:?}"
            );
        }
    });
}

#[test]
fn t5_run_turn_for_session_wires_created_cwd_into_local_cwd() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let mut sdk = CapturingSdk::default();
        let input = TurnInput {
            session_id: "sess_wire".into(),
            message: "hello".into(),
        };

        let body = cursor_adapter::run_turn_for_session(&mut sdk, &input).expect("run");
        assert_eq!(body, "cursor-sdk-ok");
        assert_eq!(sdk.creates.len(), 1);
        assert_eq!(sdk.run_count, 1);

        let cwd = session_cwd::session_cwd_for("sess_wire").expect("cwd allocated");
        assert_eq!(sdk.creates[0].local_cwd, cwd);
        assert_dir_exists(&cwd);
        // A1 combo: same Agent.create carries mcpServers + local.cwd.
        let mcp = r#loop::session_capability_mcp_config().expect("mcp");
        assert_eq!(
            sdk.creates[0].mcp_servers,
            cursor_adapter::map_mcp_config_to_mcp_servers(&mcp)
        );
    });
}

#[test]
fn t5_create_cwd_failure_does_not_start_cursor_turn_or_tools_dispatch() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let created = todo_task::create_master_with_subs("t5-no-dispatch", Some(&["sub"]));
        assert_eq!(created["_status"], 201);
        let master = created["master_task_id"].as_str().unwrap().to_string();
        let title_before = todo_task::get_by_id(&master)["title"].clone();

        session_cwd::force_create_fail_for_tests(true);
        let mut sdk = CapturingSdk::default();
        let input = TurnInput {
            session_id: "sess_no_start".into(),
            message: "must not run".into(),
        };
        let err = cursor_adapter::run_turn_for_session(&mut sdk, &input)
            .expect_err("cwd create failure must fail the turn");
        assert!(
            matches!(err, CursorAdapterError::Cwd(_)),
            "expected Cwd error, got {err:?}"
        );
        assert!(
            sdk.creates.is_empty() && sdk.run_count == 0,
            "must not Agent.create / run_turn when cwd allocation fails"
        );
        // Failure must not fall back to process-local tools::dispatch.
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);

        // Source guard: session_cwd + failure path must not import tools dispatch.
        let cwd_src = include_str!("../../../services/agent/session_cwd.rs");
        assert!(
            !cwd_src.contains("tools::dispatch")
                && !cwd_src.contains("crate::services::agent::tools"),
            "session_cwd must not call tools::dispatch"
        );
        session_cwd::force_create_fail_for_tests(false);
    });
}

#[test]
fn t5_session_cwd_module_isolated_from_host_loop_imports() {
    let loop_src = include_str!("../../../services/agent/loop.rs");
    let llm_src = include_str!("../../../services/agent/llm.rs");
    for (label, src) in [("loop.rs", loop_src), ("llm.rs", llm_src)] {
        assert!(
            !src.contains("session_cwd"),
            "{label} must not import session_cwd (Cursor-path lifecycle)"
        );
    }
}
