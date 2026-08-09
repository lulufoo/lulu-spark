//! T-Engine / Phase-Ensure: CursorLlmEngine → ensure_client → request(request_id, …).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    self, CursorError, CursorErrorCode, CursorLlmEngine, CursorRunnerClient, FakeCursorRunnerClient,
    FakeLogEntry, TurnRequest,
};
use crate::services::agent::engine_router::{self, EngineKind, EngineRuntimeConfig};
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
    TurnRequest {
        session_id: session_id.into(),
        prompt: prompt.into(),
        model: "composer-1".into(),
        api_key: "sk-engine-test".into(),
        ready_mcp: ready_mcp_sample(),
    }
}

fn methods(log: &Mutex<Vec<FakeLogEntry>>) -> Vec<String> {
    log.lock()
        .unwrap()
        .iter()
        .map(|e| e.method.clone())
        .collect()
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

        let out = engine
            .run_turn(&turn_req("sess_a", "hello"))
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

        engine
            .run_turn(&turn_req("sess_reuse", "one"))
            .expect("turn1");
        engine
            .run_turn(&turn_req("sess_reuse", "two"))
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
fn t3_foreground_switch_submits_create_only_no_host_dispose_create() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));
        let engine = CursorLlmEngine::with_manager(mgr);

        engine.run_turn(&turn_req("sess_fg1", "a")).expect("fg1");
        engine.run_turn(&turn_req("sess_fg2", "b")).expect("fg2");
        assert_eq!(spawns.load(Ordering::SeqCst), 1, "switch must keep Node");

        let m = methods(&log);
        assert_eq!(
            m.iter().filter(|x| *x == "create").count(),
            2,
            "each foreground gets create(new context): {m:?}"
        );
        assert!(
            !m.iter().any(|x| x == "close"),
            "Host/Engine must not orchestrate dispose→create via close: {m:?}"
        );
        let create_idxs: Vec<_> = m
            .iter()
            .enumerate()
            .filter(|(_, x)| *x == "create")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(create_idxs.len(), 2);
        let between = &m[create_idxs[0]..create_idxs[1]];
        assert!(
            !between.iter().any(|x| x == "cancel" || x == "close"),
            "between creates Host must not dispose; runner owns replace: {m:?}"
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
        let out = engine
            .run_turn(&turn_req("sess_switch", "after-cursor"))
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

        let err = engine
            .run_turn(&turn_req("sess_fail", "x"))
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
