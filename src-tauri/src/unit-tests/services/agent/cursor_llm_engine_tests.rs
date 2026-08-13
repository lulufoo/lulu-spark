//! T-Engine / Phase-Ensure: CursorLlmEngine → ensure_client → request(request_id, …).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::{fs, path::PathBuf};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    self, CreateRequest, CursorError, CursorErrorCode, CursorLlmEngine, CursorRunnerClient,
    FakeCursorRunnerClient, FakeLogEntry, TurnRequest,
};
use crate::services::agent::engine_router::{self, EngineKind, EngineRuntimeConfig};
use crate::services::agent::profile::BusinessProfileSnapshot;
use crate::services::agent::process_manager::{self, CursorAgentProcessManager, EnsureError};
use crate::services::agent::session_cwd;
use crate::services::mcp_endpoint_readiness::ReadyMcpTransports;
use crate::services::mcp_server_registry::HttpMcpTransport;
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    process_manager::reset_global_for_tests();
    session_cwd::reset_for_tests();
    f();
    session_cwd::reset_for_tests();
    process_manager::reset_global_for_tests();
    secrets::test_secrets_clear();
}

fn settings_cursor() -> AppSettings {
    let mut s = AppSettings::default();
    s.assistant_engine = "cursor".into();
    settings::upsert_llm_entry(
        &mut s.llm,
        "cursor",
        &LlmSettings {
            model: "composer-1".into(),
            ..Default::default()
        },
    )
    .expect("upsert cursor");
    s
}

fn settings_host() -> AppSettings {
    let mut s = AppSettings::default();
    s.assistant_engine = "host".into();
    s
}

fn cfg_from(settings: &AppSettings) -> EngineRuntimeConfig {
    engine_router::read_engine_runtime_config(settings).expect("runtime config")
}

fn manager_with_factory<F>(settings: AppSettings, factory: F) -> CursorAgentProcessManager
where
    F: FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send + 'static,
{
    let settings = Arc::new(Mutex::new(settings));
    CursorAgentProcessManager::with_deps_for_tests(
        move || {
            let s = settings.lock().unwrap_or_else(|e| e.into_inner()).clone();
            Ok(cfg_from(&s))
        },
        factory,
    )
}

fn counting_factory(
    spawn_count: Arc<AtomicUsize>,
    log: Arc<Mutex<Vec<FakeLogEntry>>>,
) -> impl FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send {
    move |key: &str| {
        spawn_count.fetch_add(1, Ordering::SeqCst);
        let mut c = FakeCursorRunnerClient::from_shared(log.clone());
        c.on_spawned_with_api_key(key);
        Ok(Box::new(c) as Box<dyn CursorRunnerClient>)
    }
}

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

fn turn_req(session_id: &str, prompt: &str) -> TurnRequest {
    let profile = super::test_business_profile("composer-1");
    fs::create_dir_all(&profile.cwd).expect("test profile cwd");
    TurnRequest {
        session_id: session_id.into(),
        prompt: prompt.into(),
        api_key: "sk-engine-test".into(),
        profile,
    }
}

fn methods(log: &Mutex<Vec<FakeLogEntry>>) -> Vec<String> {
    log.lock()
        .unwrap()
        .iter()
        .map(|e| e.method.clone())
        .collect()
}

fn t5_profile(business_id: &str, model: &str, label: &str) -> (BusinessProfileSnapshot, PathBuf) {
    let cwd = std::env::temp_dir().join(format!(
        "t5-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&cwd).expect("profile cwd");
    let mut profile = super::test_business_profile(model);
    profile.business_id = business_id.to_string();
    profile.cwd = cwd.clone();
    (profile, cwd)
}

fn t5_create_request(
    business_id: &str,
    session_id: &str,
    profile: BusinessProfileSnapshot,
) -> CreateRequest {
    CreateRequest {
        business_id: business_id.to_string(),
        session_id: session_id.to_string(),
        profile,
        api_key: "sk-engine-test".to_string(),
    }
}

fn t5_turn_request(
    profile: BusinessProfileSnapshot,
    session_id: &str,
    prompt: &str,
) -> TurnRequest {
    TurnRequest {
        session_id: session_id.to_string(),
        prompt: prompt.to_string(),
        api_key: "sk-engine-test".to_string(),
        profile,
    }
}

fn t5_create_for_turn(req: &TurnRequest) -> CreateRequest {
    CreateRequest {
        business_id: req.profile.business_id.clone(),
        session_id: req.session_id.clone(),
        profile: req.profile.clone(),
        api_key: req.api_key.clone(),
    }
}

// ── Surface / Access 合同 ────────────────────────────────────────────────────

#[test]
fn t3_cursor_llm_engine_type_lives_in_cursor_adapter() {
    let _ = std::any::type_name::<CursorLlmEngine>();
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        src.contains("struct CursorLlmEngine"),
        "CursorLlmEngine must live in cursor_adapter.rs (T-Engine)"
    );
    assert!(
        src.contains("ensure_client"),
        "engine path must call ensure_client"
    );
    assert!(
        src.contains("request("),
        "engine path must submit request(request_id, method, params)"
    );
}

#[test]
fn t3_client_access_exposes_request_with_caller_request_id() {
    let pm = include_str!("../../../services/agent/process_manager.rs");
    assert!(
        pm.contains("fn request")
            && pm.contains("request_id")
            && pm.contains("impl ClientAccess"),
        "ClientAccess must expose request(request_id, method, params)"
    );
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        adapter.contains("fn request") && adapter.contains("request_id"),
        "ProcessCursorRunnerClient / CursorRunnerClient must accept caller request_id"
    );
}

#[test]
fn t4_process_client_has_pending_map_demux_bridge() {
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        adapter.contains("struct JsonlBridge")
            && adapter.contains("pending")
            && adapter.contains("concurrent_jsonl"),
        "ProcessCursorRunnerClient must demux via JsonlBridge pending map"
    );
    let pm = include_str!("../../../services/agent/process_manager.rs");
    assert!(
        pm.contains("concurrent_jsonl") && pm.contains("submit"),
        "ClientAccess must submit via concurrent JSONL handle without holding wait lock"
    );
}

#[test]
fn t4_client_access_allows_overlapping_requests_via_concurrent_jsonl() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        fake.set_turn_block(Duration::from_millis(200));
        let log = fake.log.clone();
        let mgr = manager_with_factory(
            settings_cursor(),
            counting_factory(Arc::new(AtomicUsize::new(0)), log.clone()),
        );
        mgr.warm().expect("warm");
        let access = mgr.ensure_client().expect("ensure");

        let access_a = access;
        // Re-ensure same generation for second handle (same Ready entry).
        let access_b = mgr.ensure_client().expect("ensure b");

        let start = Instant::now();
        let t1 = thread::spawn(move || {
            access_a
                .request("rid-slow", "turn", Some(serde_json::json!({ "prompt": "slow" })))
                .expect("slow turn")
        });
        thread::sleep(Duration::from_millis(20));
        let t2 = thread::spawn(move || {
            access_b
                .request(
                    "rid-fast",
                    "create",
                    Some(serde_json::json!({
                        "session_id": "s",
                        "model": "m",
                        "cwd": "/tmp",
                    })),
                )
                .expect("overlapping create")
        });
        let _ = t2.join().expect("create thread");
        let _ = t1.join().expect("turn thread");
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "overlapping requests must not fully serialize on client mutex; took {elapsed:?}"
        );
        let methods: Vec<_> = log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert!(
            methods.iter().any(|m| m == "turn") && methods.iter().any(|m| m == "create"),
            "both methods recorded: {methods:?}"
        );
    });
}

#[test]
fn t3_access_contract_only_cursor_llm_engine_calls_ensure_client() {
    for (label, src) in [
        ("loop.rs", include_str!("../../../services/agent/loop.rs")),
        ("llm.rs", include_str!("../../../services/agent/llm.rs")),
        ("session.rs", include_str!("../../../services/agent/session.rs")),
        (
            "host_startup.rs",
            include_str!("../../../services/agent/host_startup.rs"),
        ),
    ] {
        assert!(
            !src.contains("ensure_client"),
            "{label} must not call ensure_client (Access 合同: only CursorLlmEngine)"
        );
    }
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        adapter.contains("ensure_client"),
        "CursorLlmEngine wiring must call ensure_client"
    );
    let loop_src = include_str!("../../../services/agent/loop.rs");
    assert!(
        !loop_src.contains("ensure_client") && !loop_src.contains("CursorAgentProcessManager"),
        "Todos/业务入口 must only rebind; no ensure"
    );
}

#[test]
fn t3_engine_must_not_construct_process_client_directly() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    let start = src
        .find("struct CursorLlmEngine")
        .expect("CursorLlmEngine struct");
    let after = &src[start..];
    let end = after
        .find("pub struct CursorSessionRuntime")
        .unwrap_or(after.len());
    let engine_src = &after[..end];
    assert!(
        !engine_src.contains("ProcessCursorRunnerClient::spawn")
            && !engine_src.contains("ProcessCursorRunnerClient::new"),
        "CursorLlmEngine must not bypass manager to construct ProcessCursorRunnerClient"
    );
}

// ── Happy: ensure → request(request_id) ──────────────────────────────────────

#[test]
fn t3_run_turn_ensures_client_then_requests_with_upper_request_id() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));
        let engine = CursorLlmEngine::with_manager(mgr);

        let req = turn_req("sess_a", "hello");
        engine
            .create(&t5_create_for_turn(&req))
            .expect("create");
        let out = engine
            .run_turn(&req)
            .expect("run_turn");
        assert_eq!(out.text, "fake-ok");
        assert_eq!(spawns.load(Ordering::SeqCst), 1, "managed runner once");

        let entries = log.lock().unwrap().clone();
        assert!(
            entries.iter().any(|e| e.method == "create"),
            "first turn must create: {:?}",
            methods(&log)
        );
        assert!(
            entries.iter().any(|e| e.method == "turn"),
            "first turn must request turn: {:?}",
            methods(&log)
        );
        for e in &entries {
            if e.method == "create" || e.method == "turn" {
                let rid = e
                    .request_id
                    .as_deref()
                    .expect("request_id must be recorded for JSONL id");
                assert!(!rid.is_empty(), "request_id must be non-empty");
                assert!(
                    rid.starts_with("eng-"),
                    "request_id must come from CursorLlmEngine (eng-*), not client next_id; got {rid}"
                );
            }
        }
    });
}

#[test]
fn t3_same_session_multi_turn_reuses_managed_runner_no_respawn() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));
        let engine = CursorLlmEngine::with_manager(mgr);

        let first = turn_req("sess_reuse", "one");
        engine
            .create(&t5_create_for_turn(&first))
            .expect("create");
        engine
            .run_turn(&first)
            .expect("turn1");
        let second = turn_req("sess_reuse", "two");
        engine
            .run_turn(&second)
            .expect("turn2");
        assert_eq!(
            spawns.load(Ordering::SeqCst),
            1,
            "multi-turn must reuse managed runner"
        );

        let m = methods(&log);
        let creates = m.iter().filter(|x| *x == "create").count();
        let turns = m.iter().filter(|x| *x == "turn").count();
        assert_eq!(creates, 1, "same session: one create, got {m:?}");
        assert_eq!(turns, 2, "same session: two turns, got {m:?}");
        assert!(
            !m.iter().any(|x| x == "close" || x == "cancel"),
            "same-session multi-turn must not dispose/cancel runner: {m:?}"
        );
    });
}

#[test]
fn t3_process_generation_change_forces_create_for_same_session() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-gen-v1").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let spawns_c = spawns.clone();
        let log_c = log.clone();
        let mgr = Arc::new(CursorAgentProcessManager::with_deps_for_tests(
            {
                let s = settings_cursor();
                move || Ok(cfg_from(&s))
            },
            move |key: &str| {
                spawns_c.fetch_add(1, Ordering::SeqCst);
                let mut c = FakeCursorRunnerClient::from_shared(log_c.clone());
                // Reclaim/force_kill leaves cancel_requested on shared fake; clear on respawn.
                c.reset_lifecycle_flags_for_tests();
                c.on_spawned_with_api_key(key);
                Ok(Box::new(c) as Box<dyn CursorRunnerClient>)
            },
        ));
        let engine = CursorLlmEngine::with_shared_manager(mgr.clone());

        let before = turn_req("sess_gen", "before");
        engine
            .create(&t5_create_for_turn(&before))
            .expect("before create");
        engine
            .run_turn(&before)
            .expect("before");
        assert_eq!(
            methods(&log).iter().filter(|x| *x == "create").count(),
            1
        );

        // API key fingerprint replace bumps process generation.
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-gen-v2").expect("key2");
        let after_key = turn_req("sess_gen", "after-key");
        engine
            .create(&t5_create_for_turn(&after_key))
            .expect("after-key create");
        engine
            .run_turn(&after_key)
            .expect("after key replace");
        assert_eq!(spawns.load(Ordering::SeqCst), 2);
        assert_eq!(
            methods(&log).iter().filter(|x| *x == "create").count(),
            2,
            "same session_id must re-create after process generation change"
        );

        // invalidate → Absent → next ensure rebuilds generation again.
        mgr.invalidate();
        let after_invalidate = turn_req("sess_gen", "after-invalidate");
        engine
            .create(&t5_create_for_turn(&after_invalidate))
            .expect("after-invalidate create");
        engine
            .run_turn(&after_invalidate)
            .expect("after invalidate");
        assert_eq!(spawns.load(Ordering::SeqCst), 3);
        assert_eq!(
            methods(&log).iter().filter(|x| *x == "create").count(),
            3,
            "same session_id must re-create after invalidate/rebuild"
        );
    });
}

#[test]
fn t3_foreground_switch_submits_create_only_no_host_dispose_create() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));
        let engine = CursorLlmEngine::with_manager(mgr);

        let first = turn_req("sess_fg1", "a");
        engine
            .create(&t5_create_for_turn(&first))
            .expect("create");
        engine.run_turn(&first).expect("fg1");
        let second = turn_req("sess_fg2", "b");
        engine.run_turn(&second).expect("fg2");
        assert_eq!(spawns.load(Ordering::SeqCst), 1, "switch must keep Node");

        let m = methods(&log);
        assert_eq!(
            m.iter().filter(|x| *x == "create").count(),
            1,
            "same business slot gets one explicit create: {m:?}"
        );
        assert!(
            !m.iter().any(|x| x == "close"),
            "Host/Engine must not orchestrate dispose→create via close: {m:?}"
        );
        assert_eq!(
            m.iter().filter(|x| *x == "turn").count(),
            2,
            "both UI sessions turn through the same business slot: {m:?}"
        );
    });
}

#[test]
fn t3_shared_client_request_api_has_no_session_id_serialization_key() {
    let pm = include_str!("../../../services/agent/process_manager.rs");
    let req_sig_ok = pm.contains("fn request")
        && pm.contains("request_id")
        && !pm
            .lines()
            .any(|l| l.contains("fn request") && l.contains("session_id"));
    assert!(
        req_sig_ok,
        "ClientAccess::request must not take session_id as a serialization key"
    );
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        adapter.lines().any(|l| {
            l.contains("fn request") && l.contains("request_id") && !l.contains("session_id")
        }),
        "client request signature must be (request_id, method, params)"
    );
}

// ── Boundary: engine config resolve / cold start ─────────────────────────────

#[test]
fn t3_each_send_resolves_cursor_engine_cold_start_via_ensure() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let settings_slot = Arc::new(Mutex::new(settings_host()));
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let settings_for_cfg = settings_slot.clone();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            move || {
                let s = settings_for_cfg
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                Ok(cfg_from(&s))
            },
            counting_factory(spawns.clone(), log.clone()),
        );
        let engine = CursorLlmEngine::with_manager(mgr);

        let host_cfg = cfg_from(&settings_host());
        assert_eq!(host_cfg.engine, EngineKind::Host);

        // Switch engine to Cursor without recreating business session.
        *settings_slot.lock().unwrap() = settings_cursor();
        let req = turn_req("sess_switch", "after-cursor");
        engine
            .create(&t5_create_for_turn(&req))
            .expect("cold create");
        let out = engine
            .run_turn(&req)
            .expect("cold start after engine switch");
        assert_eq!(out.text, "fake-ok");
        assert_eq!(
            spawns.load(Ordering::SeqCst),
            1,
            "return to Cursor must cold-start ensure/spawn once"
        );
        assert!(
            methods(&log).iter().any(|m| m == "create"),
            "cold start send must create Agent"
        );
    });
}

// ── Exception: ensure failure recoverable, no replay ─────────────────────────

#[test]
fn t3_ensure_failure_is_recoverable_without_auto_replay() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_c = attempts.clone();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            || Ok(cfg_from(&settings_cursor())),
            move |_key: &str| {
                attempts_c.fetch_add(1, Ordering::SeqCst);
                Err(CursorError::new(
                    CursorErrorCode::Runner,
                    cursor_adapter::frontend_message_for(CursorErrorCode::Runner),
                ))
            },
        );
        let engine = CursorLlmEngine::with_manager(mgr);

        let req = turn_req("sess_fail", "x");
        let err = engine
            .create(&t5_create_for_turn(&req))
            .expect_err("ensure/spawn failure must surface");
        assert_ne!(
            err.message,
            cursor_adapter::HOST_GENERIC_UPSTREAM_UNAVAILABLE
        );
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            1,
            "must not auto-replay ensure/spawn on failure"
        );
        assert!(
            matches!(
                err.code,
                CursorErrorCode::Runner | CursorErrorCode::Credential | CursorErrorCode::SdkConfig
            ),
            "unexpected code {:?}",
            err.code
        );
        let _ = EnsureError::Spawn;
    });
}

#[test]
fn t3_runtime_cursor_path_uses_cursor_llm_engine_not_per_session_spawn() {
    let runtime = include_str!("../../../services/agent/runtime.rs");
    assert!(
        runtime.contains("CursorLlmEngine"),
        "runtime Cursor path must wire CursorLlmEngine (Phase-Ensure)"
    );
    assert!(
        runtime.contains("CursorLlmEngine")
            && (runtime.contains(".run_turn(") || runtime.contains("run_turn(&")),
        "runtime must invoke CursorLlmEngine run_turn"
    );
}

// ── T-P3: businessId dispatch gate ──────────────────────────────────────────

#[test]
fn t5_engine_routes_create_turn_cancel_close_by_business_id() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let manager = manager_with_factory(
            settings_cursor(),
            counting_factory(Arc::new(AtomicUsize::new(0)), log.clone()),
        );
        let engine = CursorLlmEngine::with_manager(manager);
        let (profile, cwd) = t5_profile("todo_task", "composer-1", "dispatch");

        engine
            .create(&t5_create_request("todo_task", "ui-session-a", profile.clone()))
            .expect("create");
        engine
            .run_turn(&t5_turn_request(
                profile,
                "ui-session-b",
                "hello",
            ))
            .expect("turn");
        engine.cancel("todo_task").expect("cancel");
        engine.close("todo_task").expect("close");

        let entries = log.lock().unwrap().clone();
        for method in ["create", "turn", "cancel", "close"] {
            let entry = entries
                .iter()
                .find(|entry| entry.method == method)
                .unwrap_or_else(|| panic!("expected {method} dispatch: {entries:?}"));
            let params = entry.params.as_ref().expect("request params");
            assert_eq!(
                params.get("business_id").and_then(|value| value.as_str()),
                Some("todo_task"),
                "{method} must dispatch by business_id: {params}"
            );
        }

        let create = entries
            .iter()
            .find(|entry| entry.method == "create")
            .expect("create entry");
        let params = create.params.as_ref().expect("create params");
        assert_eq!(
            params.get("session_id").and_then(|value| value.as_str()),
            Some("ui-session-a"),
            "session_id remains create metadata"
        );
        assert_eq!(
            params.get("model").and_then(|value| value.as_str()),
            Some("composer-1")
        );
        assert_eq!(
            params.get("cwd").and_then(|value| value.as_str()),
            Some(cwd.to_string_lossy().as_ref()),
            "create must use the caller Profile cwd"
        );
        assert!(params.get("mcpServers").is_some());
        assert!(
            session_cwd::session_cwd_for("ui-session-a").is_none(),
            "business create must not allocate session cwd"
        );
        let _ = fs::remove_dir_all(cwd);
    });
}

#[test]
fn t5_missing_business_id_hard_fails_without_runner_dispatch_or_session_fallback() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let manager = manager_with_factory(
            settings_cursor(),
            counting_factory(spawns.clone(), log.clone()),
        );
        let engine = CursorLlmEngine::with_manager(manager);
        let (profile, cwd) = t5_profile("todo_task", "composer-1", "missing-business");

        let error = engine
            .create(&t5_create_request("", "session-only", profile.clone()))
            .expect_err("missing create business_id must fail");
        assert_ne!(error.code, CursorErrorCode::Cancelled);

        let mut turn_profile = profile;
        turn_profile.business_id.clear();
        let error = engine
            .run_turn(&t5_turn_request(turn_profile, "session-only", "must fail"))
            .expect_err("missing turn business_id must fail");
        assert_ne!(error.code, CursorErrorCode::Cancelled);

        assert!(engine.cancel("").is_err());
        assert!(engine.close("   ").is_err());
        assert_eq!(spawns.load(Ordering::SeqCst), 0);
        assert!(
            log.lock().unwrap().is_empty(),
            "session_id must never be used as a fallback dispatch key: {:?}",
            methods(&log)
        );
        let _ = fs::remove_dir_all(cwd);
    });
}

#[test]
fn t5_incomplete_create_profile_hard_fails_without_profile_backfill_or_runner_dispatch() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let manager = manager_with_factory(
            settings_cursor(),
            counting_factory(Arc::new(AtomicUsize::new(0)), log.clone()),
        );
        let engine = CursorLlmEngine::with_manager(manager);
        let (mut profile, cwd) = t5_profile("todo_task", "composer-1", "missing-profile");

        profile.model.clear();
        let error = engine
            .create(&t5_create_request("todo_task", "session-profile", profile.clone()))
            .expect_err("incomplete Profile must fail");
        assert_ne!(error.code, CursorErrorCode::Cancelled);
        assert!(
            log.lock().unwrap().is_empty(),
            "incomplete Profile must not reach Runner"
        );

        profile.model = "composer-1".to_string();
        profile.mcp_servers.clear();
        let error = engine
            .create(&t5_create_request("todo_task", "session-profile", profile))
            .expect_err("missing MCP Profile section must fail");
        assert_ne!(error.code, CursorErrorCode::Cancelled);
        assert!(
            log.lock().unwrap().is_empty(),
            "engine must not self-read or fabricate a missing Profile"
        );
        let _ = fs::remove_dir_all(cwd);
    });
}

#[test]
fn t5_engine_has_no_session_foreground_binding_or_multi_business_cache() {
    let source = include_str!("../../../services/agent/cursor_adapter.rs");
    let start = source
        .find("pub struct CursorLlmEngine")
        .expect("CursorLlmEngine struct");
    let after = &source[start..];
    let end = after
        .find("\nfn log_cursor_timing")
        .unwrap_or(after.len());
    let engine_source = &after[..end];

    assert!(
        !engine_source.contains("ForegroundBind")
            && !engine_source.contains("foreground:")
            && !engine_source.contains("bind.session_id"),
        "engine must not keep a session-keyed foreground Agent binding"
    );
    assert!(
        !engine_source.contains("HashMap")
            && !engine_source.contains("BTreeMap")
            && !engine_source.contains("profile_cache")
            && !engine_source.contains("agents:"),
        "engine must not cache multi-business Agents or Profiles"
    );
    assert!(
        !engine_source.contains("create_session_cwd")
            && !engine_source.contains("cleanup_session_cwd"),
        "business cwd must come from Profile, not session cwd lifecycle"
    );
    assert!(
        engine_source.contains("\"business_id\"")
            && engine_source.contains("\"create\"")
            && engine_source.contains("\"turn\"")
            && engine_source.contains("\"cancel\"")
            && engine_source.contains("\"close\""),
        "engine must expose business_id dispatch for every Runner lifecycle operation"
    );
}
