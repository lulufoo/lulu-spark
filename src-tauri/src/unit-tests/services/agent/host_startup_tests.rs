//! T-Warm: Host startup coordinator wires `warm()` (Cursor+key) / explicit skip.

use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    CursorError, CursorErrorCode, CursorLlmEngine, CursorRunnerClient, FakeCursorRunnerClient,
    TurnRequest,
};
use crate::services::agent::diagnostics;
use crate::services::agent::engine_router::{self, EngineKind, EngineRuntimeConfig};
use crate::services::agent::host_startup::{self, WarmCoordOutcome};
use crate::services::agent::profile::{BusinessProfileSnapshot, ProfileQueryError};
use crate::services::agent::process_manager::{
    self, CursorAgentProcessManager, ProcessLifecycleState, WarmError,
};
use crate::test_support::TestSandbox;
use serde_json::{json, Value};

fn with_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    process_manager::reset_global_for_tests();
    f();
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
    log: Arc<Mutex<Vec<crate::services::agent::cursor_adapter::FakeLogEntry>>>,
) -> impl FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send {
    move |key: &str| {
        spawn_count.fetch_add(1, Ordering::SeqCst);
        let mut c = FakeCursorRunnerClient::from_shared(log.clone());
        c.on_spawned_with_api_key(key);
        Ok(Box::new(c) as Box<dyn CursorRunnerClient>)
    }
}

// ── Module / wiring surface ──────────────────────────────────────────────────

#[test]
fn t2_host_startup_module_exists_under_agent() {
    let _ = std::any::type_name::<WarmCoordOutcome>();
    let src = include_str!("../../../services/agent/host_startup.rs");
    assert!(
        src.contains("fn coordinate_warm"),
        "Host startup coordinator must expose coordinate_warm"
    );
    assert!(
        src.contains("fn coordinate_warm_for"),
        "testable gate coordinate_warm_for required"
    );
    assert!(
        src.contains("fn schedule_cursor_runner_warm"),
        "startup must expose non-blocking schedule_cursor_runner_warm"
    );
    assert!(
        !src.contains("ProcessCursorRunnerClient::spawn")
            && !src.contains("ProcessCursorRunnerClient::new"),
        "coordinator must not bypass manager to construct/spawn ProcessCursorRunnerClient"
    );
    assert!(
        src.contains(".warm()") || src.contains("warm()"),
        "coordinator must go through manager.warm()"
    );
}

#[test]
fn t2_lib_setup_wires_schedule_cursor_runner_warm() {
    let lib = include_str!("../../../lib.rs");
    assert!(
        lib.contains("schedule_cursor_runner_warm"),
        "Host setup must call schedule_cursor_runner_warm (Phase-Warm)"
    );
    let setup_idx = lib
        .find(".setup(|app|")
        .expect("tauri setup closure required");
    let setup_tail = &lib[setup_idx..];
    let setup_end = setup_tail
        .find(".build(")
        .expect("setup followed by build");
    let setup_body = &setup_tail[..setup_end];
    assert!(
        setup_body.contains("schedule_cursor_runner_warm"),
        "warm schedule must live inside Host setup"
    );
    assert!(
        !setup_body.contains("ProcessCursorRunnerClient::spawn"),
        "setup must not bypass manager to spawn ProcessCursorRunnerClient"
    );
}

// ── Happy: Cursor + key → warm ───────────────────────────────────────────────

#[test]
fn t2_coordinate_warm_cursor_with_key_calls_warm_runner_one_no_create() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t2-warm").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let warm_calls = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));
        let cfg = cfg_from(&settings_cursor());
        assert_eq!(cfg.engine, EngineKind::Cursor);
        assert!(cfg.credential.as_deref().is_some_and(|k| !k.is_empty()));

        let warm_calls_cb = warm_calls.clone();
        let outcome = host_startup::coordinate_warm_for(&cfg, || {
            warm_calls_cb.fetch_add(1, Ordering::SeqCst);
            mgr.warm()
        });

        assert_eq!(outcome, WarmCoordOutcome::Warmed);
        assert_eq!(warm_calls.load(Ordering::SeqCst), 1, "must call warm()");
        assert_eq!(mgr.managed_runner_count_for_tests(), 1);
        assert_eq!(spawns.load(Ordering::SeqCst), 1);
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
        let methods: Vec<String> = log
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.method.clone())
            .collect();
        assert!(
            !methods.iter().any(|m| m == "create"),
            "warm must not create SDK Agent; log={methods:?}"
        );
        assert_eq!(
            fake.last_spawn_api_key.lock().unwrap().as_deref(),
            Some("sk-t2-warm")
        );
        assert_eq!(
            host_startup::last_warm_outcome_for_tests(),
            Some(WarmCoordOutcome::Warmed)
        );
    });
}

// ── Boundary: no key / non-Cursor ────────────────────────────────────────────

#[test]
fn t2_coordinate_warm_cursor_without_key_skips_explicitly_runner_zero() {
    with_sandbox(|| {
        let fake = FakeCursorRunnerClient::new();
        let spawns = Arc::new(AtomicUsize::new(0));
        let warm_calls = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), fake.log));
        let cfg = cfg_from(&settings_cursor());
        assert!(cfg.credential.is_none());

        let warm_calls_cb = warm_calls.clone();
        let outcome = host_startup::coordinate_warm_for(&cfg, || {
            warm_calls_cb.fetch_add(1, Ordering::SeqCst);
            mgr.warm()
        });

        assert_eq!(outcome, WarmCoordOutcome::SkippedNoApiKey);
        assert_eq!(
            warm_calls.load(Ordering::SeqCst),
            0,
            "no key: must not call warm()/spawn path"
        );
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        assert_eq!(spawns.load(Ordering::SeqCst), 0);
        assert_eq!(
            host_startup::last_warm_outcome_for_tests(),
            Some(WarmCoordOutcome::SkippedNoApiKey)
        );
    });
}

#[test]
fn t2_coordinate_warm_non_cursor_does_not_call_warm_runner_zero() {
    with_sandbox(|| {
        let fake = FakeCursorRunnerClient::new();
        let spawns = Arc::new(AtomicUsize::new(0));
        let warm_calls = Arc::new(AtomicUsize::new(0));
        let mgr = manager_with_factory(settings_host(), counting_factory(spawns.clone(), fake.log));
        let cfg = cfg_from(&settings_host());
        assert_eq!(cfg.engine, EngineKind::Host);

        let warm_calls_cb = warm_calls.clone();
        let outcome = host_startup::coordinate_warm_for(&cfg, || {
            warm_calls_cb.fetch_add(1, Ordering::SeqCst);
            mgr.warm()
        });

        assert_eq!(outcome, WarmCoordOutcome::SkippedNonCursor);
        assert_eq!(
            warm_calls.load(Ordering::SeqCst),
            0,
            "non-Cursor: must not call warm()"
        );
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        assert_eq!(spawns.load(Ordering::SeqCst), 0);
        assert_eq!(
            host_startup::last_warm_outcome_for_tests(),
            Some(WarmCoordOutcome::SkippedNonCursor)
        );
    });
}

// ── Failure: warm Err must not abort Host ────────────────────────────────────

#[test]
fn t2_coordinate_warm_failure_is_soft_app_continues() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t2-fail").expect("key");
        let warm_calls = Arc::new(AtomicUsize::new(0));
        let cfg = cfg_from(&settings_cursor());
        let warm_calls_cb = warm_calls.clone();

        let outcome = host_startup::coordinate_warm_for(&cfg, || {
            warm_calls_cb.fetch_add(1, Ordering::SeqCst);
            Err(WarmError::Spawn)
        });

        assert_eq!(outcome, WarmCoordOutcome::Failed);
        assert_eq!(warm_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            host_startup::last_warm_outcome_for_tests(),
            Some(WarmCoordOutcome::Failed)
        );
    });
}

#[test]
fn t2_coordinate_warm_production_entry_skips_without_key() {
    with_sandbox(|| {
        settings::save(&settings_cursor()).expect("save settings");
        let outcome = host_startup::coordinate_warm(CursorAgentProcessManager::global());
        assert_eq!(outcome, WarmCoordOutcome::SkippedNoApiKey);
        assert_eq!(
            CursorAgentProcessManager::global().managed_runner_count_for_tests(),
            0
        );
        assert_eq!(
            host_startup::last_warm_outcome_for_tests(),
            Some(WarmCoordOutcome::SkippedNoApiKey)
        );
    });
}

#[test]
fn t2_schedule_cursor_runner_warm_returns_without_joining() {
    with_sandbox(|| {
        let start = std::time::Instant::now();
        host_startup::schedule_cursor_runner_warm();
        assert!(
            start.elapsed() < std::time::Duration::from_millis(500),
            "schedule_cursor_runner_warm must not block Host setup on warm"
        );
    });
}

fn t6_profile(label: &str) -> BusinessProfileSnapshot {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let cwd = std::env::temp_dir().join(format!("lulu-t6-{label}-{}-{suffix}", std::process::id()));
    fs::create_dir_all(&cwd).expect("business cwd");
    let mut profile = super::test_business_profile("composer-1");
    profile.cwd = cwd;
    profile
}

fn t6_diagnostic_events() -> Vec<Value> {
    let path = diagnostics::diagnostic_log_path().expect("diagnostic path");
    fs::read_to_string(path)
        .expect("diagnostic log")
        .lines()
        .map(|line| serde_json::from_str(line).expect("diagnostic JSON"))
        .collect()
}

fn t6_last_diagnostic(event: &str) -> Value {
    t6_diagnostic_events()
        .into_iter()
        .rev()
        .find(|value| value["event"].as_str() == Some(event))
        .unwrap_or_else(|| panic!("missing diagnostic event {event}"))
}

struct FailingCreateClient {
    error: CursorError,
}

impl CursorRunnerClient for FailingCreateClient {
    fn close(&mut self) -> Result<(), CursorError> {
        Ok(())
    }

    fn force_kill(&mut self) {}

    fn request(
        &mut self,
        _request_id: &str,
        method: &str,
        _params: Option<Value>,
    ) -> Result<Value, CursorError> {
        if method == "create" {
            Err(self.error.clone())
        } else {
            Ok(json!({}))
        }
    }
}

struct BlockingCreateState {
    create_started: AtomicBool,
    release_create: AtomicBool,
    create_finished: AtomicBool,
    turn_before_create_finished: AtomicBool,
    methods: Mutex<Vec<String>>,
}

struct BlockingCreateClient {
    state: Arc<BlockingCreateState>,
}

impl CursorRunnerClient for BlockingCreateClient {
    fn close(&mut self) -> Result<(), CursorError> {
        Ok(())
    }

    fn force_kill(&mut self) {
        self.state.release_create.store(true, Ordering::SeqCst);
    }

    fn request(
        &mut self,
        _request_id: &str,
        method: &str,
        _params: Option<Value>,
    ) -> Result<Value, CursorError> {
        self.state
            .methods
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(method.to_string());
        match method {
            "create" => {
                self.state.create_started.store(true, Ordering::SeqCst);
                while !self.state.release_create.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(2));
                }
                self.state.create_finished.store(true, Ordering::SeqCst);
                Ok(json!({}))
            }
            "turn" => {
                if !self.state.create_finished.load(Ordering::SeqCst) {
                    self.state
                        .turn_before_create_finished
                        .store(true, Ordering::SeqCst);
                }
                Ok(json!({ "text": "turn-after-prewarm" }))
            }
            _ => Ok(json!({})),
        }
    }
}

#[test]
fn t6_prewarm_queries_default_profile_and_creates_through_engine() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t6-prewarm").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let manager =
            manager_with_factory(settings_cursor(), counting_factory(Arc::new(AtomicUsize::new(0)), log.clone()));
        manager.warm().expect("runner warm");
        let engine = CursorLlmEngine::with_manager(manager);
        let profile = t6_profile("success");
        let queried = Arc::new(Mutex::new(Vec::<String>::new()));
        let queried_for_call = queried.clone();
        let profile_for_call = profile.clone();

        host_startup::prewarm_default_todos_agent_for(&engine, move |business_id| {
            queried_for_call
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(business_id.to_string());
            assert_eq!(business_id, "todo_task");
            Ok(profile_for_call)
        })
        .expect("default Todos prewarm");

        assert_eq!(
            queried.lock().unwrap_or_else(|error| error.into_inner()).as_slice(),
            ["todo_task"]
        );
        let entries = log.lock().unwrap_or_else(|error| error.into_inner()).clone();
        let create = entries
            .iter()
            .find(|entry| entry.method == "create")
            .expect("engine create");
        let params = create.params.as_ref().expect("create params");
        assert_eq!(params["business_id"], "todo_task");
        assert_eq!(
            params["session_id"],
            host_startup::DEFAULT_TODOS_PREWARM_SESSION_ID
        );
        assert_ne!(params["session_id"], "ui-session");
        assert_eq!(params["model"], profile.model);
        assert_eq!(params["cwd"], profile.cwd.to_string_lossy().as_ref());
        assert!(params.get("mcpServers").is_some(), "Profile MCP servers required");

        let diagnostic = t6_last_diagnostic("prewarm_create_ok");
        assert_eq!(diagnostic["fields"]["business_id"], "todo_task");
        assert!(!diagnostic["trace_id"].as_str().unwrap_or("").is_empty());
        let _ = fs::remove_dir_all(profile.cwd);
    });
}

#[test]
fn t6_profile_failure_is_nonfatal_and_keeps_business_retry_ownership() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t6-profile-fail").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let spawns = Arc::new(AtomicUsize::new(0));
        let manager = manager_with_factory(
            settings_cursor(),
            counting_factory(spawns.clone(), fake.log.clone()),
        );
        let engine = CursorLlmEngine::with_manager(manager);
        let error = host_startup::prewarm_default_todos_agent_for(&engine, |_business_id| {
            Err(ProfileQueryError::Settings("profile unavailable".into()))
        })
        .expect_err("Profile failure must be reported");

        assert!(matches!(error, host_startup::PrewarmError::Profile(_)));
        assert_eq!(
            spawns.load(Ordering::SeqCst),
            0,
            "Profile failure must not spawn or fall back to another Agent"
        );
        let diagnostic = t6_last_diagnostic("prewarm_create_fail");
        assert_eq!(diagnostic["fields"]["business_id"], "todo_task");
    });
}

#[test]
fn t6_create_failure_is_diagnosed_and_later_ensure_create_retries() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t6-create-fail").expect("key");
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_for_factory = attempts.clone();
        let manager = manager_with_factory(settings_cursor(), move |_key| {
            if attempts_for_factory.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(Box::new(FailingCreateClient {
                    error: CursorError::new(CursorErrorCode::Runner, "forced create failure"),
                }) as Box<dyn CursorRunnerClient>)
            } else {
                Ok(Box::new(FakeCursorRunnerClient::new()) as Box<dyn CursorRunnerClient>)
            }
        });
        manager.warm().expect("runner warm");
        let engine = CursorLlmEngine::with_manager(manager);
        let profile = t6_profile("create-failure");
        let profile_for_failure = profile.clone();
        let error = host_startup::prewarm_default_todos_agent_for(&engine, move |_business_id| {
            Ok(profile_for_failure)
        })
        .expect_err("create failure must be reported");
        assert!(matches!(error, host_startup::PrewarmError::Create(_)));

        let failed = t6_last_diagnostic("prewarm_create_fail");
        assert_eq!(failed["fields"]["business_id"], "todo_task");

        let profile_for_retry = profile.clone();
        host_startup::prewarm_default_todos_agent_for(&engine, move |_business_id| {
            Ok(profile_for_retry)
        })
        .expect("later ensure-create retry");
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        let succeeded = t6_last_diagnostic("prewarm_create_ok");
        assert_eq!(succeeded["fields"]["business_id"], "todo_task");
        let _ = fs::remove_dir_all(profile.cwd);
    });
}

#[test]
fn t6_first_business_turn_waits_for_the_inflight_prewarm_create() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t6-inflight").expect("key");
        let state = Arc::new(BlockingCreateState {
            create_started: AtomicBool::new(false),
            release_create: AtomicBool::new(false),
            create_finished: AtomicBool::new(false),
            turn_before_create_finished: AtomicBool::new(false),
            methods: Mutex::new(Vec::new()),
        });
        let state_for_factory = state.clone();
        let manager = manager_with_factory(settings_cursor(), move |_key| {
            Ok(Box::new(BlockingCreateClient {
                state: state_for_factory.clone(),
            }) as Box<dyn CursorRunnerClient>)
        });
        manager.warm().expect("runner warm");
        let engine = Arc::new(CursorLlmEngine::with_manager(manager));
        let profile = t6_profile("inflight");
        let profile_for_prewarm = profile.clone();
        let prewarm_engine = engine.clone();
        let prewarm_thread = thread::spawn(move || {
            host_startup::prewarm_default_todos_agent_for(&prewarm_engine, move |_business_id| {
                Ok(profile_for_prewarm)
            })
        });

        let wait_start = std::time::Instant::now();
        while !state.create_started.load(Ordering::SeqCst) {
            assert!(
                wait_start.elapsed() < Duration::from_secs(1),
                "prewarm create did not start"
            );
            thread::sleep(Duration::from_millis(2));
        }

        let turn_engine = engine.clone();
        let turn_profile = profile.clone();
        let turn_thread = thread::spawn(move || {
            turn_engine.run_turn(&TurnRequest {
                session_id: "ui-session".into(),
                prompt: "first business request".into(),
                api_key: "sk-t6-inflight".into(),
                profile: turn_profile,
            })
        });
        thread::sleep(Duration::from_millis(50));
        assert!(
            !state.create_finished.load(Ordering::SeqCst),
            "test must observe create still in progress"
        );
        assert!(
            !turn_thread.is_finished(),
            "first business request must wait for the same create"
        );

        state.release_create.store(true, Ordering::SeqCst);
        prewarm_thread
            .join()
            .expect("prewarm thread")
            .expect("prewarm result");
        let turn = turn_thread.join().expect("turn thread").expect("turn result");
        assert_eq!(turn.text, "turn-after-prewarm");
        assert!(!state.turn_before_create_finished.load(Ordering::SeqCst));
        assert_eq!(
            state
                .methods
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .as_slice(),
            ["create", "turn"]
        );
        let _ = fs::remove_dir_all(profile.cwd);
    });
}

#[test]
fn t6_startup_delegates_agent_create_to_engine_after_runner_warm() {
    let source = include_str!("../../../services/agent/host_startup.rs");
    let coordinator_start = source
        .find("fn coordinate_warm_with_trace")
        .expect("production warm coordinator");
    let coordinator = &source[coordinator_start..];
    let coordinator_end = coordinator.find("\n}\n\n/// Fire-and-forget").expect("coordinator end");
    let coordinator = &coordinator[..coordinator_end];
    let warm = coordinator
        .find("coordinate_warm_for")
        .expect("warm gate");
    let prewarm = coordinator
        .find("prewarm_default_todos_agent")
        .expect("prewarm gate");
    assert!(prewarm > warm, "Profile/create must happen after Runner warm");
    assert!(
        source.contains("CursorLlmEngine")
            && source.contains(".create("),
        "startup prewarm must route Agent create through CursorLlmEngine"
    );
    assert!(
        !source.contains("access.request") && !source.contains("ReadyMcpTransports"),
        "startup must not directly create on Runner or add an MCP readiness gate"
    );
}
