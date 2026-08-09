//! T4 + T3: Cursor Local adapter — SDK shape, production process client, lifecycle.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;

use crate::services::agent::cursor_adapter::{
    self, AgentCreateParams, CursorAdapterError, CursorErrorCode, CursorSessionRuntime,
    FakeCursorRunnerClient, TurnRequest,
};
use crate::services::agent::engine_router::{self, AdapterKind, EngineKind, TurnInput};
use crate::services::agent::r#loop;
use crate::services::agent::session_cwd;
use crate::services::mcp_endpoint_readiness::{self, ReadyMcpTransports};
use crate::services::mcp_server_registry::{
    self, HttpMcpTransport, McpServerConfig, SEEDED_BUSINESS_KEY,
};
use crate::services::todo_task;
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

fn key_only_payload(key: &str) -> serde_json::Value {
    json!({ "key": key })
}

fn sample_cwd() -> PathBuf {
    PathBuf::from("/tmp/cursor-adapter-t4-placeholder-cwd")
}

/// Capturing double for the replaceable Cursor Agent SDK Local port (A2).
#[derive(Default)]
struct CapturingSdk {
    creates: Vec<AgentCreateParams>,
    fail_create: bool,
    fail_run: bool,
}

impl cursor_adapter::CursorAgentSdk for CapturingSdk {
    fn create(&mut self, params: AgentCreateParams) -> Result<(), CursorAdapterError> {
        self.creates.push(params);
        if self.fail_create {
            Err(CursorAdapterError::Sdk("create failed".into()))
        } else {
            Ok(())
        }
    }

    fn run_turn(&mut self, _message: &str) -> Result<String, CursorAdapterError> {
        if self.fail_run {
            Err(CursorAdapterError::Sdk("run_turn failed".into()))
        } else {
            Ok("cursor-sdk-ok".into())
        }
    }
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

#[test]
fn t4_mcp_config_maps_to_mcp_servers_inline_shape() {
    let config = McpServerConfig {
        capability_description: "internal host-mcp todo_task capability surface".into(),
        http_transport: crate::services::mcp_server_registry::HttpMcpTransport {
            name: "workbench".into(),
            url: "http://127.0.0.1:9876/mcp".into(),
            headers: Default::default(),
        },
    };
    let servers = cursor_adapter::map_mcp_config_to_mcp_servers(&config);
    let v = serde_json::to_value(&servers).expect("serialize mcpServers");
    // Agent SDK Local inline mcpServers: named server entry with decision-level config.
    assert!(
        v.as_object().map(|o| !o.is_empty()).unwrap_or(false),
        "mcpServers must be a non-empty object: {v}"
    );
    let entry = v
        .as_object()
        .and_then(|o| o.values().next())
        .expect("at least one server entry");
    assert_eq!(
        entry.get("capability_description").and_then(|x| x.as_str()),
        Some(config.capability_description.as_str())
    );
}

#[test]
fn t4_create_agent_injects_mcp_servers_and_local_cwd_placeholder() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let mcp = r#loop::session_capability_mcp_config().expect("L2 read face");
        let cwd = sample_cwd();
        let mut sdk = CapturingSdk::default();

        cursor_adapter::create_agent(&mut sdk, &mcp, cwd.clone()).expect("create");

        assert_eq!(sdk.creates.len(), 1);
        let params = &sdk.creates[0];
        assert_eq!(params.local_cwd, cwd, "local.cwd injection point reserved for t5");
        let expected = cursor_adapter::map_mcp_config_to_mcp_servers(&mcp);
        assert_eq!(params.mcp_servers, expected);
    });
}

#[test]
fn t4_run_turn_reads_session_capability_mcp_config_and_captures_create_params() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let before = r#loop::session_capability_mcp_config().expect("loaded");
        let cwd = sample_cwd();
        let mut sdk = CapturingSdk::default();
        let input = TurnInput {
            session_id: "sess_t4".into(),
            message: "hello cursor".into(),
        };

        let body = cursor_adapter::run_turn(&mut sdk, &input, cwd.clone()).expect("run");
        assert_eq!(body, "cursor-sdk-ok");
        assert_eq!(sdk.creates.len(), 1);
        assert_eq!(sdk.creates[0].local_cwd, cwd);
        assert_eq!(
            sdk.creates[0].mcp_servers,
            cursor_adapter::map_mcp_config_to_mcp_servers(&before)
        );
        // Read face must remain unchanged (read-only consumption).
        let after = r#loop::session_capability_mcp_config().expect("still loaded");
        assert_eq!(after, before);
    });
}

#[test]
fn t4_settings_cursor_route_enters_cursor_adapter_module() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let cwd = sample_cwd();
        let sdk = Arc::new(Mutex::new(CapturingSdk::default()));
        let sdk_host = sdk.clone();
        let input = TurnInput {
            session_id: "sess_route_t4".into(),
            message: "route me".into(),
        };

        let out = engine_router::route_chat_turn(
            EngineKind::Cursor,
            &input,
            || Ok("host-must-not-run".into()),
            || {
                let mut guard = sdk_host.lock().expect("sdk lock");
                cursor_adapter::run_turn(&mut *guard, &input, cwd.clone())
                    .map_err(|e| e.to_string())
            },
        )
        .expect("cursor route");

        assert_eq!(out.adapter, AdapterKind::Cursor);
        assert_eq!(out.body, "cursor-sdk-ok");
        assert_eq!(sdk.lock().unwrap().creates.len(), 1);
    });
}

#[test]
fn t4_failure_path_does_not_call_tools_dispatch() {
    with_sandbox(|| {
        let created = todo_task::create_master_with_subs("t4-no-dispatch", Some(&["sub"]));
        assert_eq!(created["_status"], 201);
        let master = created["master_task_id"].as_str().unwrap().to_string();
        let title_before = todo_task::get_by_id(&master)["title"].clone();

        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let mut sdk = CapturingSdk {
            fail_create: true,
            ..CapturingSdk::default()
        };
        let input = TurnInput {
            session_id: "sess_fail".into(),
            message: "force fail".into(),
        };
        let err = cursor_adapter::run_turn(&mut sdk, &input, sample_cwd())
            .expect_err("SDK create failure must surface");
        assert!(
            matches!(err, CursorAdapterError::Sdk(_)),
            "expected Sdk error, got {err:?}"
        );
        // Cursor failure must not fall back to process-local tools (L09-I #7).
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
    });
}

#[test]
fn t4_missing_session_mcp_config_fails_without_tools_dispatch() {
    with_sandbox(|| {
        // No Binding Set → read face empty.
        assert!(r#loop::session_capability_mcp_config().is_none());
        let created = todo_task::create_master_with_subs("t4-no-mcp", Some(&["s"]));
        let master = created["master_task_id"].as_str().unwrap().to_string();
        let title_before = todo_task::get_by_id(&master)["title"].clone();

        let mut sdk = CapturingSdk::default();
        let input = TurnInput {
            session_id: "sess_no_mcp".into(),
            message: "no config".into(),
        };
        let err = cursor_adapter::run_turn(&mut sdk, &input, sample_cwd())
            .expect_err("missing MCP config must fail");
        assert!(
            matches!(err, CursorAdapterError::MissingMcpConfig),
            "expected MissingMcpConfig, got {err:?}"
        );
        assert!(
            sdk.creates.is_empty(),
            "must not Agent.create without mcpServers"
        );
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
    });
}

#[test]
fn t4_run_turn_sdk_failure_does_not_fallback_to_process_tools() {
    with_sandbox(|| {
        r#loop::try_set_binding_json(&key_only_payload(SEEDED_BUSINESS_KEY)).expect("Set");
        let created = todo_task::create_master_with_subs("t4-sdk-run-fail", Some(&["s"]));
        let master = created["master_task_id"].as_str().unwrap().to_string();
        let title_before = todo_task::get_by_id(&master)["title"].clone();

        let mut sdk = CapturingSdk {
            fail_run: true,
            ..CapturingSdk::default()
        };
        let input = TurnInput {
            session_id: "sess_run_fail".into(),
            message: "run fail".into(),
        };
        let err = cursor_adapter::run_turn(&mut sdk, &input, sample_cwd())
            .expect_err("run_turn SDK failure");
        assert!(matches!(err, CursorAdapterError::Sdk(_)));
        assert_eq!(sdk.creates.len(), 1, "create may succeed before run fails");
        assert_eq!(todo_task::get_by_id(&master)["title"], title_before);
    });
}

// ── T3: production process client + lifecycle ────────────────────────────────

fn ready_mcp_sample() -> ReadyMcpTransports {
    let mut headers = std::collections::BTreeMap::new();
    headers.insert(
        "Accept".into(),
        "application/json, text/event-stream".into(),
    );
    ReadyMcpTransports {
        transports: vec![HttpMcpTransport {
            name: "workbench".into(),
            url: "http://127.0.0.1:9876/mcp".into(),
            headers,
        }],
    }
}

fn t3_turn_request(session_id: &str, prompt: &str) -> TurnRequest {
    TurnRequest {
        session_id: session_id.into(),
        prompt: prompt.into(),
        model: "composer-2.5".into(),
        api_key: "sk-test-cursor-key".into(),
        ready_mcp: ready_mcp_sample(),
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
        let expected = mcp_endpoint_readiness::map_to_sdk_mcp_servers(&ready_mcp_sample());
        let expected_v = serde_json::to_value(&expected).unwrap();
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
