//! T4 + T3: Cursor Local adapter — module isolation, production process client, lifecycle.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::services::agent::cursor_adapter::{
    self, CursorErrorCode, CursorSessionRuntime, FakeCursorRunnerClient, TurnRequest,
};
use crate::services::agent::diagnostics;
use crate::services::agent::r#loop;
use crate::services::agent::session_cwd;
use crate::services::mcp_server_registry;
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();
    r#loop::reset_runtime_for_tests();
    f();
    r#loop::reset_runtime_for_tests();
    mcp_server_registry::clear_for_tests();
}

#[test]
fn t4_cursor_adapter_module_is_isolated_from_host_loop() {
    // Cursor adapter is a sibling module; Host loop/llm must not link it.
    let loop_src = include_str!("../../../services/agent/loop.rs");
    let llm_src = include_str!("../../../services/agent/llm.rs");
    for (label, src) in [("loop.rs", loop_src), ("llm.rs", llm_src)] {
        assert!(
            !src.contains("cursor_adapter"),
            "{label} must not import cursor_adapter (Host path zero Cursor)"
        );
    }
    // Adapter itself must not call process-local tools::dispatch.
    let adapter_src = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        !adapter_src.contains("tools::dispatch")
            && !adapter_src.contains("crate::services::agent::tools"),
        "cursor_adapter must not call tools::dispatch / import tools"
    );
    assert!(
        cursor_adapter::CURSOR_SDK_A2_REPLACEABLE_PORT,
        "A2 must be explicit: SDK embed via replaceable port (real crate unavailable)"
    );
}

// ── T3: production process client + lifecycle ────────────────────────────────

fn t3_turn_request(session_id: &str, prompt: &str) -> TurnRequest {
    TurnRequest {
        session_id: session_id.into(),
        prompt: prompt.into(),
        api_key: "sk-test-cursor-key".into(),
        profile: super::test_business_profile("composer-2.5"),
    }
}

#[test]
fn t3_production_adapter_uses_real_process_client_type() {
    // Production surface must expose ProcessCursorRunnerClient (not only trait/test double).
    let _ = std::any::type_name::<cursor_adapter::ProcessCursorRunnerClient>();
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        src.contains("struct ProcessCursorRunnerClient"),
        "production process client must exist"
    );
    assert!(
        src.contains("CURSOR_API_KEY"),
        "API key must be injected via child env CURSOR_API_KEY"
    );
    assert!(
        !src.contains("tools::dispatch")
            && !src.contains("crate::services::agent::tools"),
        "cursor_adapter must not call tools::dispatch"
    );
    assert!(
        !src.contains("resume"),
        "must not use SDK resume path that drops inline MCP"
    );
}

#[test]
fn t3_production_runner_captures_stderr_and_logs_lifecycle_boundaries() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        src.contains(".stderr(Stdio::piped())"),
        "production Runner stderr must be captured for durable diagnostics"
    );
    for marker in [
        "cursor.runner.spawned",
        "cursor.runner.stderr",
        "cursor.runner.stdout_eof",
        "cursor.runner.close.started",
        "cursor.runner.force_kill.started",
        "cursor.runner.turn_empty",
        "text_len",
        "sdk_status",
        "sdk_error_message",
        "sdk_error_code",
        "sdk_wait_json",
        "stderr_preview",
        "with_bounded_text_field",
    ] {
        assert!(
            src.contains(marker),
            "production Runner diagnostics must include {marker}"
        );
    }
}

#[test]
fn t3_runner_stderr_diagnostic_preserves_business_id_in_durable_log() {
    with_sandbox(|| {
        cursor_adapter::log_runner_stderr_line_for_tests(
            4242,
            r#"{"event":"turn_fail","business_id":"todos","error_type":"sdk_run"}"#,
        );

        let path = diagnostics::diagnostic_log_path().expect("diagnostic path");
        let contents = fs::read_to_string(path).expect("diagnostic log");
        let line = contents
            .lines()
            .last()
            .expect("diagnostic line");
        let event: Value = serde_json::from_str(line).expect("diagnostic JSON");
        assert_eq!(event["event"], "cursor.runner.turn_fail");
        assert_eq!(event["fields"]["business_id"], "todos");
    });
}

#[test]
fn t3_typed_runner_errors_map_to_frontend_safe_cursor_codes() {
    let host_generic = cursor_adapter::HOST_GENERIC_UPSTREAM_UNAVAILABLE;
    assert_eq!(host_generic, "上游服务暂时不可用，请稍后重试。");

    let cases = [
        ("credential", CursorErrorCode::Credential),
        ("sdk_config", CursorErrorCode::SdkConfig),
        ("mcp_unavailable", CursorErrorCode::McpUnavailable),
        ("cwd", CursorErrorCode::Cwd),
        ("runner", CursorErrorCode::Runner),
        ("sdk_run", CursorErrorCode::SdkRun),
        ("cancelled", CursorErrorCode::Cancelled),
    ];
    for (runner_type, expected) in cases {
        let err = cursor_adapter::map_runner_error(runner_type, "detail");
        assert_eq!(err.code, expected, "type {runner_type}");
        assert_ne!(
            err.message, host_generic,
            "{runner_type} must not collapse to Host generic upstream message"
        );
        assert!(
            !err.message.contains("sk-") && !err.message.contains("CURSOR_API_KEY"),
            "frontend message must not leak secrets: {}",
            err.message
        );
    }
}

#[test]
fn t3_fake_client_reuses_same_agent_across_turns_without_resume() {
    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let runtime = CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        });

        let r1 = runtime
            .run_turn(&t3_turn_request("sess_reuse", "first"))
            .expect("turn1");
        assert!(r1.should_persist);
        assert_eq!(r1.text, "fake-ok");

        let r2 = runtime
            .run_turn(&t3_turn_request("sess_reuse", "second"))
            .expect("turn2");
        assert!(r2.should_persist);

        let methods: Vec<String> = fake
            .log
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert_eq!(
            methods,
            vec!["create".to_string(), "turn".to_string(), "turn".to_string()],
            "create once then reuse agent via turn; no resume: {methods:?}"
        );
        let cwd = session_cwd::session_cwd_for("sess_reuse").expect("cwd reused");
        assert!(cwd.is_dir());
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t3_one_session_one_in_flight_turn() {
    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        fake.set_turn_block(Duration::from_millis(200));
        let log = fake.log.clone();
        let runtime = Arc::new(CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        }));

        let rt_a = runtime.clone();
        let handle = std::thread::spawn(move || rt_a.run_turn(&t3_turn_request("sess_busy", "slow")));
        std::thread::sleep(Duration::from_millis(30));
        let err = runtime
            .run_turn(&t3_turn_request("sess_busy", "second"))
            .expect_err("second in-flight turn must fail");
        assert_eq!(err.code, CursorErrorCode::Busy);
        let _ = handle.join().expect("join");
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t3_cancel_for_binding_cut_waits_terminal_and_does_not_persist() {
    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        fake.set_turn_block(Duration::from_millis(300));
        let log = fake.log.clone();
        let runtime = Arc::new(CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        }));

        let rt_a = runtime.clone();
        let turn_handle =
            std::thread::spawn(move || rt_a.run_turn(&t3_turn_request("sess_cut", "in-flight")));
        std::thread::sleep(Duration::from_millis(30));

        // Reset / replacement Set / defensive unbound all use cancel-first.
        runtime
            .cancel_for_binding_cut("sess_cut")
            .expect("cancel");

        let turn_result = turn_handle.join().expect("join");
        match turn_result {
            Ok(r) => assert!(
                !r.should_persist,
                "cancelled/expired results must not persist to Assistant session"
            ),
            Err(e) => {
                assert_eq!(e.code, CursorErrorCode::Cancelled);
                assert!(!cursor_adapter::should_persist_cursor_result(&Err(e)));
            }
        }
        let methods: Vec<_> = fake
            .log
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert!(
            methods.iter().any(|m| m == "cancel"),
            "binding cut must send cancel: {methods:?}"
        );
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t3_cancel_timeout_force_kills_child() {
    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        fake.set_cancel_hang(true);
        let log = fake.log.clone();
        let runtime = CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        });
        runtime.set_cancel_timeout(Duration::from_millis(50));

        runtime
            .run_turn(&t3_turn_request("sess_kill", "x"))
            .expect("create+turn");
        // Simulate in-flight then cancel with hanging cancel().
        fake.mark_in_flight(true);
        runtime
            .cancel_for_binding_cut("sess_kill")
            .expect("cancel path completes via force kill");
        assert!(
            fake.force_killed.load(std::sync::atomic::Ordering::SeqCst),
            "timeout must force-kill child"
        );
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t3_close_awaits_dispose_then_cleans_cwd_shell_close_does_not() {
    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let runtime = CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        });

        runtime
            .run_turn(&t3_turn_request("sess_close", "hi"))
            .expect("turn");
        let cwd = session_cwd::session_cwd_for("sess_close").expect("cwd");
        assert!(cwd.is_dir());

        // Shell close: session continues — must NOT clean cwd/agent.
        runtime.on_shell_close("sess_close");
        assert!(
            cwd.is_dir(),
            "shell close must retain cwd (existing semantics)"
        );
        assert!(
            runtime.has_session("sess_close"),
            "shell close must retain agent session"
        );
        let methods_after_shell: Vec<_> = fake
            .log
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert!(
            !methods_after_shell.iter().any(|m| m == "close"),
            "shell close must not send close/dispose"
        );

        // Explicit close: Node must await agent dispose before responding; then Rust cleans cwd.
        runtime.close_session("sess_close").expect("close");
        let methods: Vec<_> = fake
            .log
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert!(
            methods.iter().any(|m| m == "close"),
            "close must call runner close (Node awaits Symbol.asyncDispose): {methods:?}"
        );
        assert!(
            fake.dispose_awaited.load(std::sync::atomic::Ordering::SeqCst),
            "close must await dispose on Node side before ok"
        );
        assert!(
            !cwd.exists(),
            "Rust cleans cwd only after successful close"
        );
        assert!(!runtime.has_session("sess_close"));
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t3_close_paths_cover_reset_replacement_defensive_and_app_exit() {
    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let runtime = CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        });

        for sid in ["sess_reset", "sess_replace", "sess_defensive", "sess_exit"] {
            runtime
                .run_turn(&t3_turn_request(sid, "hi"))
                .expect("turn");
        }

        // Reset / replacement Set / defensive cut: cancel then close+cleanup.
        for sid in ["sess_reset", "sess_replace", "sess_defensive"] {
            runtime.cancel_for_binding_cut(sid).expect("cancel");
            runtime.close_session(sid).expect("close");
            assert!(session_cwd::session_cwd_for(sid).is_none());
        }

        // App exit closes remaining sessions.
        runtime.close_all_for_app_exit().expect("app exit");
        assert!(!runtime.has_session("sess_exit"));
        assert!(session_cwd::session_cwd_for("sess_exit").is_none());
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t3_create_uses_ready_mcp_servers_shape_and_keeps_key_out_of_jsonl() {
    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let runtime = CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        });

        runtime
            .run_turn(&t3_turn_request("sess_mcp", "hi"))
            .expect("turn");

        let entries = fake.log.lock().unwrap().clone();
        let create = entries
            .iter()
            .find(|e| e.method == "create")
            .expect("create logged");
        let params = create.params.as_ref().expect("create params");
        assert!(
            params.get("apiKey").is_none() && params.get("api_key").is_none(),
            "API key must not appear in JSONL params: {params}"
        );
        let mcp = params.get("mcpServers").expect("mcpServers");
        let expected_profile = super::test_business_profile("composer-2.5");
        let expected_v = serde_json::to_value(&expected_profile.mcp_servers).unwrap();
        assert_eq!(mcp, &expected_v);
        // Factory receives key for env injection only.
        assert_eq!(
            fake.last_spawn_api_key.lock().unwrap().as_deref(),
            Some("sk-test-cursor-key")
        );
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t3_cancelled_result_helper_blocks_assistant_persist() {
    let cancelled = Err(cursor_adapter::CursorError {
        code: CursorErrorCode::Cancelled,
        message: "x".into(),
    });
    assert!(!cursor_adapter::should_persist_cursor_result(&cancelled));
    let ok = Ok(cursor_adapter::TurnOutcome {
        text: "hi".into(),
        should_persist: true,
    });
    assert!(cursor_adapter::should_persist_cursor_result(&ok));
    let ok_but_flagged = Ok(cursor_adapter::TurnOutcome {
        text: "stale".into(),
        should_persist: false,
    });
    assert!(!cursor_adapter::should_persist_cursor_result(&ok_but_flagged));
}

// ── T1 / PortNarrow: CursorRunnerClient typed surface → request only ─────────

fn extract_brace_block<'a>(src: &'a str, marker: &str) -> &'a str {
    let start = src
        .find(marker)
        .unwrap_or_else(|| panic!("missing marker: {marker}"));
    let bytes = src.as_bytes();
    let mut i = start + marker.len();
    while i < bytes.len() && bytes[i] != b'{' {
        i += 1;
    }
    assert!(i < bytes.len(), "no opening brace after {marker}");
    let begin = i;
    let mut depth = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[begin..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unclosed brace after {marker}");
}

#[test]
fn t1_cursor_runner_client_drops_typed_create_turn_cancel_keeps_six() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    let trait_body = extract_brace_block(src, "pub trait CursorRunnerClient");
    for deleted in ["fn create(", "fn turn(", "fn cancel("] {
        assert!(
            !trait_body.contains(deleted),
            "CursorRunnerClient must not declare typed {deleted}: {trait_body}"
        );
    }
    for kept in [
        "fn request(",
        "fn concurrent_jsonl(",
        "fn close(",
        "fn force_kill(",
        "fn terminate_hook(",
        "fn on_spawned_with_api_key(",
    ] {
        assert!(
            trait_body.contains(kept),
            "CursorRunnerClient must keep {kept}"
        );
    }

    for marker in [
        "impl CursorRunnerClient for FakeCursorRunnerClient",
        "impl CursorRunnerClient for ProcessCursorRunnerClient",
    ] {
        let body = extract_brace_block(src, marker);
        for deleted in ["fn create(", "fn turn(", "fn cancel("] {
            assert!(
                !body.contains(deleted),
                "{marker} must not implement typed {deleted}"
            );
        }
        for kept in ["fn request(", "fn close(", "fn force_kill(", "fn terminate_hook("] {
            assert!(body.contains(kept), "{marker} must keep {kept}");
        }
    }
}

#[test]
fn t1_csr_create_turn_cancel_go_through_request_not_typed_api() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    let csr = extract_brace_block(src, "impl CursorSessionRuntime");
    assert!(
        !csr.contains("guard.turn(") && !csr.contains(".turn(&"),
        "CSR must not call typed turn()"
    );
    assert!(
        !csr.contains("guard.cancel(") && !csr.contains(".cancel()"),
        "CSR must not call typed cancel()"
    );
    assert!(
        csr.contains("request(") && csr.contains("\"create\"") && csr.contains("\"turn\""),
        "CSR create/turn must go through request"
    );
    assert!(
        csr.contains("\"cancel\""),
        "CSR cancel path must request(\"cancel\", …)"
    );

    with_sandbox(|| {
        session_cwd::reset_for_tests();
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let runtime = CursorSessionRuntime::with_client_factory(move |key| {
            let mut c = FakeCursorRunnerClient::from_shared(log.clone());
            c.on_spawned_with_api_key(key);
            Ok(Box::new(c))
        });

        runtime
            .run_turn(&t3_turn_request("sess_port_narrow", "hi"))
            .expect("turn");
        runtime
            .cancel_for_binding_cut("sess_port_narrow")
            .expect("cancel");

        let entries = fake.log.lock().unwrap().clone();
        for method in ["create", "turn", "cancel"] {
            let hit = entries
                .iter()
                .find(|e| e.method == method)
                .unwrap_or_else(|| panic!("expected {method} log entry: {entries:?}"));
            assert!(
                hit.request_id.is_some(),
                "{method} must go through request (have request_id), got {hit:?}"
            );
        }
        session_cwd::reset_for_tests();
    });
}

#[test]
fn t1_production_engine_still_submits_create_turn_via_client_access_request() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    let engine = extract_brace_block(src, "impl CursorLlmEngine");
    assert!(
        engine.contains("request_managed(")
            && engine.contains("\"create\"")
            && engine.contains("\"turn\""),
        "CursorLlmEngine must still submit JSONL create/turn via ClientAccess::request"
    );
    assert!(
        !engine.contains(".create(")
            && !engine.contains(".turn(")
            && !engine.contains(".cancel("),
        "production engine must not call typed CursorRunnerClient create/turn/cancel"
    );
    // Lifecycle KEEP surface remains on the trait (not deleted as unused).
    let trait_body = extract_brace_block(src, "pub trait CursorRunnerClient");
    assert!(trait_body.contains("fn close("));
    assert!(trait_body.contains("fn force_kill("));
    assert!(trait_body.contains("fn terminate_hook("));
}

// ── T2 / DeleteFacade: B2a–e CursorAgentSdk surface removed ──────────────────

#[test]
fn t2_cursor_agent_sdk_facade_symbols_are_deleted() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    for deleted in [
        "pub trait CursorAgentSdk",
        "pub fn map_mcp_config_to_mcp_servers",
        "pub fn create_agent",
        "pub fn run_turn_for_session",
    ] {
        assert!(
            !src.contains(deleted),
            "B2 facade must not define {deleted}"
        );
    }
    // Sdk free-function run_turn (generic over CursorAgentSdk) — not engine/CSR methods.
    assert!(
        !src.contains("pub fn run_turn<S: CursorAgentSdk>")
            && !src.contains("pub fn run_turn<S:"),
        "B2 Sdk run_turn free function must be deleted"
    );
}

#[test]
fn t2_cursor_llm_engine_run_turn_still_exists_and_facade_not_csr() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    let engine = extract_brace_block(src, "impl CursorLlmEngine");
    assert!(
        engine.contains("pub fn run_turn("),
        "CursorLlmEngine::run_turn must remain (do not delete engine method)"
    );
    // Boundary: CSR type is REWRITE (t4), not deleted here.
    assert!(
        src.contains("struct CursorSessionRuntime") || src.contains("pub struct CursorSessionRuntime"),
        "CSR type must remain (REWRITE, not this task)"
    );
    // Boundary: do not alter ClientAccess::request / JSONL method contract via this delete.
    assert!(
        src.contains("request_managed(") && src.contains("\"create\"") && src.contains("\"turn\""),
        "ClientAccess::request / JSONL create|turn must remain after facade delete"
    );
}

// ── T3 / DeleteBEntries: B3a map_client_request_error ────────────────────────

#[test]
fn t3_map_client_request_error_deleted_keeps_private_map_request_error() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        !src.contains("fn map_client_request_error"),
        "B3a map_client_request_error must be deleted"
    );
    assert!(
        src.contains("fn map_request_error"),
        "production private map_request_error must remain"
    );
    // Do not confuse with CSR test factory.
    assert!(
        src.contains("fn with_client_factory"),
        "CursorSessionRuntime::with_client_factory is out of t3 delete set"
    );
}

// ── T4 / AlignTests: remaining doubles assert request-only surface ───────────

#[test]
fn t4_align_unit_test_doubles_drop_typed_create_turn_cancel() {
    for (label, src) in [
        (
            "error_tracks_tests.rs",
            include_str!("error_tracks_tests.rs"),
        ),
        (
            "replace_create_tests.rs",
            include_str!("replace_create_tests.rs"),
        ),
        (
            "process_manager_tests.rs",
            include_str!("process_manager_tests.rs"),
        ),
    ] {
        for deleted in ["fn create(", "fn turn(", "fn cancel("] {
            assert!(
                !src.contains(deleted),
                "{label} double must not keep typed {deleted}"
            );
        }
        assert!(
            src.contains("fn request("),
            "{label} double must implement request"
        );
    }
}

#[test]
fn t4_align_csr_still_request_only_after_facade_and_b_deletes() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    let csr = extract_brace_block(src, "impl CursorSessionRuntime");
    assert!(
        csr.contains("request(")
            && csr.contains("\"create\"")
            && csr.contains("\"turn\"")
            && csr.contains("\"cancel\""),
        "CSR must keep create/turn/cancel via request after AlignTests"
    );
    assert!(
        !csr.contains("guard.turn(")
            && !csr.contains("guard.create(")
            && !csr.contains("guard.cancel("),
        "CSR must not regress to typed create/turn/cancel"
    );
}
