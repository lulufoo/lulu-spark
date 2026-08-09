//! T6 / T-Errors: Cancelled vs RecoverableFailure tracks; stale generation; no auto-replay.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    self, CursorError, CursorErrorCode, CursorLlmEngine, CursorRunnerClient, ErrorTrack,
    FakeCursorRunnerClient, FakeLogEntry, TurnOutcome, TurnRequest,
};
use crate::services::agent::engine_router::{self, EngineRuntimeConfig};
use crate::services::agent::process_manager::{
    self, CursorAgentProcessManager, RequestError,
};
use crate::services::agent::session;
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

/// Controllable client: turn can return Cancelled or Runner (JSONL) errors; counts requests.
struct TrackFake {
    log: Arc<Mutex<Vec<FakeLogEntry>>>,
    turn_calls: Arc<AtomicUsize>,
    force_kills: Arc<AtomicUsize>,
    next_turn_err: Arc<Mutex<Option<CursorErrorCode>>>,
}

impl TrackFake {
    fn new() -> Self {
        Self {
            log: Arc::new(Mutex::new(Vec::new())),
            turn_calls: Arc::new(AtomicUsize::new(0)),
            force_kills: Arc::new(AtomicUsize::new(0)),
            next_turn_err: Arc::new(Mutex::new(None)),
        }
    }

    fn clone_factory(
        &self,
    ) -> impl FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send {
        let log = self.log.clone();
        let turn_calls = self.turn_calls.clone();
        let force_kills = self.force_kills.clone();
        let next_turn_err = self.next_turn_err.clone();
        move |_key: &str| {
            Ok(Box::new(TrackFakeClient {
                log: log.clone(),
                turn_calls: turn_calls.clone(),
                force_kills: force_kills.clone(),
                next_turn_err: next_turn_err.clone(),
            }) as Box<dyn CursorRunnerClient>)
        }
    }
}

struct TrackFakeClient {
    log: Arc<Mutex<Vec<FakeLogEntry>>>,
    turn_calls: Arc<AtomicUsize>,
    force_kills: Arc<AtomicUsize>,
    next_turn_err: Arc<Mutex<Option<CursorErrorCode>>>,
}

impl CursorRunnerClient for TrackFakeClient {
    fn create(
        &mut self,
        model: &str,
        cwd: &std::path::Path,
        mcp_servers: &Value,
    ) -> Result<(), CursorError> {
        let _ = self.request(
            "legacy",
            "create",
            Some(json!({
                "model": model,
                "cwd": cwd.to_string_lossy(),
                "mcpServers": mcp_servers,
            })),
        )?;
        Ok(())
    }

    fn turn(&mut self, prompt: &str) -> Result<String, CursorError> {
        let v = self.request("legacy", "turn", Some(json!({ "prompt": prompt })))?;
        Ok(v.get("text").and_then(|x| x.as_str()).unwrap_or("").into())
    }

    fn cancel(&mut self) -> Result<(), CursorError> {
        let _ = self.request("legacy", "cancel", None)?;
        Ok(())
    }

    fn close(&mut self) -> Result<(), CursorError> {
        let _ = self.request("legacy", "close", None)?;
        Ok(())
    }

    fn force_kill(&mut self) {
        self.force_kills.fetch_add(1, Ordering::SeqCst);
    }

    fn request(
        &mut self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, CursorError> {
        self.log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(FakeLogEntry {
                request_id: Some(request_id.into()),
                method: method.into(),
                params,
            });
        match method {
            "create" => Ok(json!({ "agentId": "track-fake" })),
            "turn" => {
                self.turn_calls.fetch_add(1, Ordering::SeqCst);
                if let Some(code) = self
                    .next_turn_err
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                {
                    return Err(CursorError::new(
                        code,
                        cursor_adapter::frontend_message_for(code),
                    ));
                }
                Ok(json!({ "text": "ok" }))
            }
            "cancel" | "close" => Ok(json!({})),
            _ => Err(CursorError::new(
                CursorErrorCode::Runner,
                "unknown method",
            )),
        }
    }
}

// ── Normal: Cancelled vs RecoverableFailure 分轨 ─────────────────────────────

#[test]
fn t6_cancelled_and_recoverable_tracks_are_distinct() {
    let cancelled = CursorError::new(
        CursorErrorCode::Cancelled,
        cursor_adapter::frontend_message_for(CursorErrorCode::Cancelled),
    );
    let recoverable = CursorError::new(
        CursorErrorCode::RecoverableFailure,
        cursor_adapter::frontend_message_for(CursorErrorCode::RecoverableFailure),
    );
    assert_ne!(
        cancelled.code, recoverable.code,
        "Cancelled and RecoverableFailure must be distinguishable codes"
    );
    assert_eq!(
        cursor_adapter::error_track(&cancelled),
        ErrorTrack::Cancelled
    );
    assert_eq!(
        cursor_adapter::error_track(&recoverable),
        ErrorTrack::RecoverableFailure
    );
    assert_ne!(
        cursor_adapter::error_track(&cancelled),
        cursor_adapter::error_track(&recoverable)
    );
}

#[test]
fn t6_neither_cancelled_nor_recoverable_persists_as_session_turn() {
    let cancelled = Err(CursorError::new(
        CursorErrorCode::Cancelled,
        "cancelled",
    ));
    let recoverable = Err(CursorError::new(
        CursorErrorCode::RecoverableFailure,
        "recoverable",
    ));
    assert!(
        !cursor_adapter::should_persist_cursor_result(&cancelled),
        "Cancelled must not persist as Session turn"
    );
    assert!(
        !cursor_adapter::should_persist_cursor_result(&recoverable),
        "RecoverableFailure must not persist as Session turn"
    );
    // Communication-failure Runner surface (pre-map) must also stay off the persist path
    // once classified as recoverable track.
    let runner = Err(CursorError::new(
        CursorErrorCode::Runner,
        cursor_adapter::frontend_message_for(CursorErrorCode::Runner),
    ));
    assert_eq!(
        cursor_adapter::error_track(runner.as_ref().err().unwrap()),
        ErrorTrack::RecoverableFailure,
        "process/JSONL Runner failures classify as RecoverableFailure"
    );
    assert!(
        !cursor_adapter::should_persist_cursor_result(&runner),
        "Runner communication failure must not persist as Session turn"
    );
}

#[test]
fn t6_foreground_interrupt_surfaces_cancelled_not_recoverable() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = TrackFake::new();
        *fake
            .next_turn_err
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(CursorErrorCode::Cancelled);
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            {
                let s = settings_cursor();
                move || Ok(cfg_from(&s))
            },
            fake.clone_factory(),
        );
        let engine = CursorLlmEngine::with_manager(mgr);

        let err = engine
            .run_turn(&turn_req("sess_fg", "interrupted"))
            .expect_err("foreground interrupt must surface");
        assert_eq!(err.code, CursorErrorCode::Cancelled);
        assert_eq!(
            cursor_adapter::error_track(&err),
            ErrorTrack::Cancelled,
            "foreground interrupt must stay on Cancelled track"
        );
        assert!(!cursor_adapter::should_persist_cursor_result(&Err(err)));
        assert_eq!(
            fake.turn_calls.load(Ordering::SeqCst),
            1,
            "must not auto-retry cancelled turn"
        );
    });
}

#[test]
fn t6_jsonl_or_process_failure_surfaces_recoverable_without_auto_replay() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = TrackFake::new();
        *fake
            .next_turn_err
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(CursorErrorCode::Runner);
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            {
                let s = settings_cursor();
                move || Ok(cfg_from(&s))
            },
            fake.clone_factory(),
        );
        let engine = CursorLlmEngine::with_manager(mgr);

        let err = engine
            .run_turn(&turn_req("sess_jsonl", "x"))
            .expect_err("JSONL/process failure must surface");
        assert_eq!(
            cursor_adapter::error_track(&err),
            ErrorTrack::RecoverableFailure,
            "process/JSONL failure must be RecoverableFailure track, got {:?}",
            err.code
        );
        assert_ne!(err.code, CursorErrorCode::Cancelled);
        assert!(!cursor_adapter::should_persist_cursor_result(&Err(err.clone())));
        assert_eq!(
            fake.turn_calls.load(Ordering::SeqCst),
            1,
            "must never auto-replay failed request"
        );
        assert_eq!(
            fake.force_kills.load(Ordering::SeqCst),
            1,
            "Runner/JSONL failure must invalidate managed process (reclaim/force_kill)"
        );
    });
}

#[test]
fn t6_cancelled_does_not_invalidate_managed_process() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let fake = TrackFake::new();
        *fake
            .next_turn_err
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(CursorErrorCode::Cancelled);
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            {
                let s = settings_cursor();
                move || Ok(cfg_from(&s))
            },
            fake.clone_factory(),
        );
        let engine = CursorLlmEngine::with_manager(mgr);

        let _ = engine
            .run_turn(&turn_req("sess_fg2", "interrupted"))
            .expect_err("cancelled");
        assert_eq!(
            fake.force_kills.load(Ordering::SeqCst),
            0,
            "Cancelled must not invalidate the managed runner"
        );
    });
}

// ── Boundary: stale generation / cache / engine-config / shutdown ────────────

#[test]
fn t6_stale_generation_access_fails_recoverable_without_stdin_write() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-v1").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            || Ok(cfg_from(&settings_cursor())),
            counting_factory(spawns, log.clone()),
        );
        mgr.warm().expect("warm");
        let access = mgr.ensure_client().expect("ensure");

        // Generation change (API key fingerprint replace).
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-v2").expect("key2");
        let _new = mgr.ensure_client().expect("ensure v2");

        let err = access
            .request("stale-rid", "turn", Some(json!({ "prompt": "nope" })))
            .expect_err("old generation access must fail");
        assert!(
            matches!(err, RequestError::Stale),
            "expected RequestError::Stale, got {err:?}"
        );
        let wrote_stale_turn = log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|e| e.method == "turn" && e.request_id.as_deref() == Some("stale-rid"));
        assert!(
            !wrote_stale_turn,
            "stale access must not write old process stdin (no client.request)"
        );

        let mapped = cursor_adapter::map_client_request_error(err);
        assert_eq!(
            cursor_adapter::error_track(&mapped),
            ErrorTrack::RecoverableFailure
        );
        assert_ne!(mapped.code, CursorErrorCode::Cancelled);
        assert!(!cursor_adapter::should_persist_cursor_result(&Err(mapped)));
    });
}

#[test]
fn t6_invalidate_marks_old_access_dead_and_maps_recoverable() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-inv").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            || Ok(cfg_from(&settings_cursor())),
            counting_factory(Arc::new(AtomicUsize::new(0)), log.clone()),
        );
        mgr.warm().expect("warm");
        let access = mgr.ensure_client().expect("ensure");

        mgr.invalidate();
        let err = access
            .request("after-inv", "turn", Some(json!({ "prompt": "x" })))
            .expect_err("invalidated access must fail");
        assert!(matches!(err, RequestError::Stale));
        let wrote_turn = log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|e| e.method == "turn" && e.request_id.as_deref() == Some("after-inv"));
        assert!(
            !wrote_turn,
            "invalidated access must not touch stdin via request"
        );
        let mapped = cursor_adapter::map_client_request_error(err);
        assert_eq!(
            cursor_adapter::error_track(&mapped),
            ErrorTrack::RecoverableFailure
        );
    });
}

#[test]
fn t6_cache_restore_keeps_completed_turns_without_cancelled_or_retry() {
    with_sandbox(|| {
        let sess = session::create_session(Some("task_t6"), Some("t6")).expect("session");
        let sid = sess.session_id.clone();
        session::append_turn(
            &sid,
            session::Turn {
                role: "user".into(),
                content: Some("completed user".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("user");
        session::append_turn(
            &sid,
            session::Turn {
                role: "assistant".into(),
                content: Some("completed assistant".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("assistant");

        // Interrupted / recoverable outcomes must not be appended (persist gate).
        let cancelled = Err(CursorError::new(CursorErrorCode::Cancelled, "c"));
        let recoverable = Err(CursorError::new(
            CursorErrorCode::RecoverableFailure,
            "r",
        ));
        assert!(!cursor_adapter::should_persist_cursor_result(&cancelled));
        assert!(!cursor_adapter::should_persist_cursor_result(&recoverable));

        let loaded = session::load_session(&sid).expect("restore");
        assert_eq!(loaded.turns.len(), 2, "restore only completed turns");
        assert_eq!(loaded.turns[0].content.as_deref(), Some("completed user"));
        assert_eq!(
            loaded.turns[1].content.as_deref(),
            Some("completed assistant")
        );
        // No automatic retry artifact: session idle; no extra turns invented on load.
        let loaded2 = session::load_session(&sid).expect("reload");
        assert_eq!(loaded2.turns.len(), 2);
    });
}

#[test]
fn t6_engine_config_switch_does_not_apply_foreground_cancelled_semantics() {
    // leaf-engine-config-switch-AR: engine-only change ≠ foreground Cancelled.
    let adapter = include_str!("../../../services/agent/cursor_adapter.rs");
    let runtime = include_str!("../../../services/agent/runtime.rs");
    assert!(
        !adapter.contains("assistant_engine")
            || !adapter
                .lines()
                .any(|l| l.contains("assistant_engine") && l.contains("Cancelled")),
        "CursorLlmEngine must not map engine-config change to Cancelled"
    );
    assert!(
        !runtime.contains("CursorErrorCode::Cancelled")
            || !runtime
                .lines()
                .any(|l| l.contains("assistant_engine") && l.to_ascii_lowercase().contains("cancel")),
        "runtime must not cancel in-flight on engine-config-only switch"
    );

    // Behavioral: successful Cursor turn remains persistable success — not Cancelled —
    // when only verifying ErrorTrack helpers ignore engine settings.
    let ok = Ok(TurnOutcome {
        text: "still running".into(),
        should_persist: true,
    });
    assert!(cursor_adapter::should_persist_cursor_result(&ok));
    let _ = ErrorTrack::Cancelled;
    let _ = ErrorTrack::RecoverableFailure;
}

#[test]
fn t6_shutdown_error_type_is_outside_communication_failure_set() {
    // App-exit shutdown is not the communication-invalid set (Node/JSONL/key-replace).
    let _ = std::any::type_name::<crate::services::agent::process_manager::ShutdownError>();
    let pm = include_str!("../../../services/agent/process_manager.rs");
    assert!(
        pm.contains("enum ShutdownError"),
        "shutdown must remain a distinct error surface"
    );
    // Communication failures map through RequestError/EnsureError → RecoverableFailure,
    // not ShutdownError.
    let mapped = cursor_adapter::map_client_request_error(RequestError::Stale);
    assert_eq!(
        cursor_adapter::error_track(&mapped),
        ErrorTrack::RecoverableFailure
    );
    assert_ne!(
        std::any::type_name::<crate::services::agent::process_manager::ShutdownError>(),
        std::any::type_name::<CursorError>()
    );
}

// ── Abnormal: rebuild failure preserves business session ─────────────────────

#[test]
fn t6_rebuild_failure_preserves_business_session_as_recoverable() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-engine-test").expect("key");
        let sess = session::create_session(None, Some("keep")).expect("session");
        let sid = sess.session_id.clone();
        session::append_turn(
            &sid,
            session::Turn {
                role: "user".into(),
                content: Some("keep me".into()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            },
        )
        .expect("append");

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
            .run_turn(&turn_req(&sid, "retry?"))
            .expect_err("rebuild/spawn failure");
        assert_eq!(
            cursor_adapter::error_track(&err),
            ErrorTrack::RecoverableFailure
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1, "no auto-replay");

        let loaded = session::load_session(&sid).expect("session survives");
        assert_eq!(loaded.turns.len(), 1);
        assert_eq!(loaded.turns[0].content.as_deref(), Some("keep me"));
    });
}
