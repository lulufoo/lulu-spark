//! T-VerifyAgent / Phase-Verify: agent suite contract + warm/turn still via `request`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    CreateRequest, CursorError, CursorLlmEngine, CursorRunnerClient, FakeCursorRunnerClient,
    FakeLogEntry, TurnRequest,
};
use crate::services::agent::engine_router::{self, EngineRuntimeConfig};
use crate::services::agent::process_manager::{self, CursorAgentProcessManager, ProcessLifecycleState};
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
    std::fs::create_dir_all(&profile.cwd).expect("test profile cwd");
    TurnRequest {
        session_id: session_id.into(),
        prompt: prompt.into(),
        api_key: "sk-verify".into(),
        profile,
    }
}

fn create_req(req: &TurnRequest) -> CreateRequest {
    CreateRequest {
        business_id: req.profile.business_id.clone(),
        session_id: req.session_id.clone(),
        profile: req.profile.clone(),
        api_key: req.api_key.clone(),
    }
}

fn extract_brace_block<'a>(src: &'a str, marker: &str) -> &'a str {
    let start = src.find(marker).unwrap_or_else(|| panic!("missing {marker}"));
    let after = &src[start + marker.len()..];
    let begin_rel = after
        .find('{')
        .unwrap_or_else(|| panic!("no '{{' after {marker}"));
    let begin = start + marker.len() + begin_rel;
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    let mut i = begin;
    while i < bytes.len() {
        match bytes[i] as char {
            '{' => depth += 1,
            '}' => {
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

fn trait_fn_names(trait_body: &str) -> Vec<String> {
    trait_body
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let rest = trimmed.strip_prefix("fn ")?;
            let name = rest.split('(').next()?.trim();
            if name.is_empty() {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

// ── AC1: delete set gone; KEEP six only ──────────────────────────────────────

#[test]
fn t5_verify_cursor_runner_client_keep_surface_locked() {
    let src = include_str!("../../../services/agent/cursor_adapter.rs");
    assert!(
        src.contains("T-VerifyAgent:"),
        "cursor_adapter.rs must carry T-VerifyAgent KEEP-lock marker after Phase-Verify"
    );

    let trait_body = extract_brace_block(src, "pub trait CursorRunnerClient");
    let keep = [
        "request",
        "concurrent_jsonl",
        "close",
        "force_kill",
        "terminate_hook",
        "on_spawned_with_api_key",
    ];
    let names = trait_fn_names(trait_body);
    assert_eq!(
        names.len(),
        keep.len(),
        "CursorRunnerClient must expose exactly KEEP six methods, got {names:?}"
    );
    for expected in keep {
        assert!(
            names.iter().any(|n| n == expected),
            "missing KEEP method {expected}; got {names:?}"
        );
    }
    for deleted in ["create", "turn", "cancel"] {
        assert!(
            !names.iter().any(|n| n == deleted),
            "typed {deleted} must not remain on CursorRunnerClient"
        );
    }
}

#[test]
fn t5_verify_ab_delete_set_symbols_absent_from_agent_sources() {
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    let pm = include_str!("../../../services/agent/process_manager.rs");
    let router = include_str!("../../../services/agent/engine_router.rs");
    let combined = format!("{adapter}\n{pm}\n{router}");

    for deleted in [
        "pub trait CursorAgentSdk",
        "pub fn map_mcp_config_to_mcp_servers",
        "pub fn create_agent",
        "pub fn run_turn_for_session",
        "fn map_client_request_error",
        "fn engine_settings_for_route",
        "fn mark_invalid_for_tests",
    ] {
        assert!(
            !combined.contains(deleted),
            "A/B delete set symbol still present: {deleted}"
        );
    }
    // B3c: ClientAccess::with_client (not CSR with_client_factory).
    let access = extract_brace_block(pm, "impl ClientAccess");
    assert!(
        !access.contains("fn with_client<") && !access.contains("fn with_client("),
        "ClientAccess::with_client must remain deleted"
    );
}

// ── AC2: ensure_client → request; JSONL create|turn contract unchanged ───────

#[test]
fn t5_verify_production_path_still_ensure_client_then_request() {
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    let engine = extract_brace_block(adapter, "impl CursorLlmEngine");
    assert!(
        engine.contains("ensure_client")
            && engine.contains("request_managed(")
            && engine.contains("\"create\"")
            && engine.contains("\"turn\""),
        "production engine must stay ensure_client → request(JSONL create|turn)"
    );
    assert!(
        !engine.contains(".create(")
            && !engine.contains(".turn(")
            && !engine.contains(".cancel("),
        "production engine must not call typed CursorRunnerClient create/turn/cancel"
    );

    let pm = include_str!("../../../services/agent/process_manager.rs");
    assert!(
        pm.contains("fn warm(") && pm.contains("fn ensure_client(") && pm.contains("fn shutdown("),
        "warm/ensure_client/shutdown surface must remain"
    );
    assert!(
        pm.contains("impl ClientAccess") && pm.contains("fn request("),
        "ClientAccess::request must remain the JSONL submit port"
    );
}

// ── AC4: warm still lifts runner; Cursor turn still via request ──────────────

#[test]
fn t5_verify_warm_then_cursor_turn_still_goes_through_request() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-verify-warm").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));

        mgr.warm().expect("warm must still lift runner client");
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
        assert_eq!(mgr.managed_runner_count_for_tests(), 1);
        assert_eq!(spawns.load(Ordering::SeqCst), 1);
        {
            let methods: Vec<String> = log
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.method.clone())
                .collect();
            assert!(
                !methods.iter().any(|m| m == "create"),
                "warm must not JSONL-create; log={methods:?}"
            );
        }

        let engine = CursorLlmEngine::with_manager(mgr);
        let req = turn_req("sess_verify", "verify turn");
        engine.create(&create_req(&req)).expect("create");
        let out = engine
            .run_turn(&req)
            .expect("cursor turn");
        assert_eq!(out.text, "fake-ok");

        let entries = log.lock().unwrap().clone();
        for method in ["create", "turn"] {
            let hit = entries
                .iter()
                .find(|e| e.method == method)
                .unwrap_or_else(|| panic!("expected {method} via request after warm: {entries:?}"));
            assert!(
                hit.request_id.as_deref().is_some_and(|id| !id.is_empty()),
                "{method} must carry request_id (ClientAccess::request), got {hit:?}"
            );
        }
    });
}
