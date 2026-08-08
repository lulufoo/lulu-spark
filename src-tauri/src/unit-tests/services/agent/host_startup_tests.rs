//! T-Warm: Host startup coordinator wires `warm()` (Cursor+key) / explicit skip.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    CursorError, CursorRunnerClient, FakeCursorRunnerClient,
};
use crate::services::agent::engine_router::{self, EngineKind, EngineRuntimeConfig};
use crate::services::agent::host_startup::{self, WarmCoordOutcome};
use crate::services::agent::process_manager::{
    self, CursorAgentProcessManager, ProcessLifecycleState, WarmError,
};
use crate::test_support::TestSandbox;

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
