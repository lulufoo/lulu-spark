//! T-Manager: CursorAgentProcessManager — warm / ensure_client / shutdown + lifecycle.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    CursorError, CursorErrorCode, CursorRunnerClient, FakeCursorRunnerClient,
};
use crate::services::agent::engine_router::{self, EngineRuntimeConfig};
use crate::services::agent::process_manager::{
    self, ClientAccess, CursorAgentProcessManager, EnsureError, ProcessLifecycleState,
    ShutdownError, WarmError,
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

fn access_is_usable(access: &ClientAccess) -> bool {
    access.with_client(|_c| ()).is_ok()
}

// ── Module / singleton / access surface ──────────────────────────────────────

#[test]
fn t1_process_manager_module_exists_under_agent() {
    let _ = std::any::type_name::<CursorAgentProcessManager>();
    let _ = std::any::type_name::<ProcessLifecycleState>();
    let src = include_str!("../../../services/agent/process_manager.rs");
    assert!(
        src.contains("struct CursorAgentProcessManager"),
        "manager type must live in process_manager.rs"
    );
    assert!(
        src.contains("fn warm") && src.contains("fn ensure_client") && src.contains("fn shutdown"),
        "public access surface warm/ensure_client/shutdown required"
    );
    assert!(
        src.contains("fn ensure_client(&self)"),
        "ensure_client must not take ProcessEntry generation as an external parameter"
    );
}

#[test]
fn t1_global_singleton_is_one_instance() {
    with_sandbox(|| {
        let a = CursorAgentProcessManager::global() as *const _;
        let b = CursorAgentProcessManager::global() as *const _;
        assert_eq!(a, b, "Workbench must expose one global manager instance");
    });
}

#[test]
fn t1_access_contract_surface_is_host_engine_exit_only_by_api() {
    // Wiring of callers is deferred (t2/t3/t7/t8); this locks the public contract.
    let src = include_str!("../../../services/agent/process_manager.rs");
    assert!(
        !src.contains("AIAssistantSession") && !src.contains("current_binding"),
        "manager must not manage business sessions / bindings"
    );
    for (label, path) in [
        ("loop.rs", include_str!("../../../services/agent/loop.rs")),
        ("llm.rs", include_str!("../../../services/agent/llm.rs")),
        ("session.rs", include_str!("../../../services/agent/session.rs")),
    ] {
        assert!(
            !path.contains("process_manager"),
            "{label} must not directly access CursorAgentProcessManager (Access 合同)"
        );
    }
}

// ── warm / ensure / shutdown happy paths ─────────────────────────────────────

#[test]
fn t1_warm_starts_process_and_client_without_sdk_create() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-warm").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));

        mgr.warm().expect("warm");
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
        assert_eq!(mgr.managed_runner_count_for_tests(), 1);
        assert_eq!(spawns.load(Ordering::SeqCst), 1);
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
            Some("sk-warm")
        );
    });
}

#[test]
fn t1_ensure_client_reuses_ready_runner_and_returns_usable_access() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-ensure").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr = manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log));

        mgr.warm().expect("warm");
        let gen1 = mgr.generation_for_tests();
        let access = mgr.ensure_client().expect("ensure");
        assert!(access_is_usable(&access));
        assert_eq!(spawns.load(Ordering::SeqCst), 1, "must reuse single runner");
        assert_eq!(mgr.generation_for_tests(), gen1);
        assert_eq!(mgr.managed_runner_count_for_tests(), 1);
    });
}

#[test]
fn t1_ensure_client_lazily_starts_when_absent() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-lazy").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr = manager_with_factory(
            settings_cursor(),
            counting_factory(spawns.clone(), fake.log.clone()),
        );

        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Absent
        );
        let access = mgr.ensure_client().expect("lazy ensure");
        assert!(access_is_usable(&access));
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
        assert_eq!(spawns.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn t1_shutdown_stops_runner_and_returns_to_absent() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-stop").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let mgr = manager_with_factory(
            settings_cursor(),
            counting_factory(Arc::new(AtomicUsize::new(0)), log.clone()),
        );
        mgr.warm().expect("warm");
        let access = mgr.ensure_client().expect("ensure");
        mgr.shutdown().expect("shutdown");
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Absent
        );
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        assert!(
            log.lock().unwrap().iter().any(|e| e.method == "close")
                || fake.force_killed.load(Ordering::SeqCst),
            "shutdown must reclaim client/process"
        );
        assert!(
            !access_is_usable(&access),
            "access after shutdown must be invalidated"
        );
    });
}

// ── boundary: non-Cursor / missing credential / key replace ──────────────────

#[test]
fn t1_non_cursor_or_missing_credential_keeps_cardinality_zero() {
    with_sandbox(|| {
        let fake = FakeCursorRunnerClient::new();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr = manager_with_factory(
            settings_host(),
            counting_factory(spawns.clone(), fake.log.clone()),
        );
        mgr.warm().expect("warm host is no-op ok");
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        assert_eq!(spawns.load(Ordering::SeqCst), 0);
        assert!(matches!(
            mgr.ensure_client().expect_err("ensure host"),
            EnsureError::Unavailable
        ));

        secrets::test_secrets_clear();
        let fake2 = FakeCursorRunnerClient::new();
        let spawns2 = Arc::new(AtomicUsize::new(0));
        let mgr2 = manager_with_factory(
            settings_cursor(),
            counting_factory(spawns2.clone(), fake2.log.clone()),
        );
        mgr2.warm().expect("warm without key skips");
        assert_eq!(mgr2.managed_runner_count_for_tests(), 0);
        assert_eq!(spawns2.load(Ordering::SeqCst), 0);
        assert!(matches!(
            mgr2.ensure_client().expect_err("ensure no key"),
            EnsureError::Unavailable
        ));
    });
}

#[test]
fn t1_api_key_fingerprint_change_replaces_via_replacing_state() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-v1").expect("key");
        let settings = Arc::new(Mutex::new(settings_cursor()));
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let settings_cfg = settings.clone();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            move || {
                let s = settings_cfg.lock().unwrap_or_else(|e| e.into_inner()).clone();
                Ok(cfg_from(&s))
            },
            counting_factory(spawns.clone(), log),
        );

        mgr.warm().expect("warm");
        let old = mgr.ensure_client().expect("ensure v1");
        let gen1 = mgr.generation_for_tests();
        assert_eq!(spawns.load(Ordering::SeqCst), 1);

        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-v2").expect("key2");

        let new_access = mgr.ensure_client().expect("ensure v2");
        assert!(access_is_usable(&new_access));
        assert!(
            !access_is_usable(&old),
            "old ClientAccess must die after generation change"
        );
        assert!(
            mgr.generation_for_tests() > gen1,
            "internal generation must advance on replace"
        );
        assert_eq!(spawns.load(Ordering::SeqCst), 2);
        assert_eq!(mgr.managed_runner_count_for_tests(), 1);
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
        assert_eq!(
            fake.last_spawn_api_key.lock().unwrap().as_deref(),
            Some("sk-v2")
        );
        let _ = settings;
    });
}

// ── abnormal: start failure / invalid → absent ───────────────────────────────

#[test]
fn t1_starting_failure_returns_absent_and_ensure_can_retry() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-fail").expect("key");
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts2 = attempts.clone();
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            || Ok(cfg_from(&settings_cursor())),
            move |key: &str| {
                let n = attempts2.fetch_add(1, Ordering::SeqCst);
                if n == 0 {
                    Err(CursorError::new(CursorErrorCode::Runner, "spawn failed"))
                } else {
                    let mut c = FakeCursorRunnerClient::from_shared(log.clone());
                    c.on_spawned_with_api_key(key);
                    Ok(Box::new(c) as Box<dyn CursorRunnerClient>)
                }
            },
        );

        let warm_err = mgr.warm().expect_err("first warm spawn fails");
        assert!(matches!(warm_err, WarmError::Spawn));
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Absent
        );
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);

        let access = mgr.ensure_client().expect("retry ensure");
        assert!(access_is_usable(&access));
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
        assert!(attempts.load(Ordering::SeqCst) >= 2);
    });
}

#[test]
fn t1_ready_invalid_marks_old_client_dead_then_absent() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-inv").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let mgr = manager_with_factory(
            settings_cursor(),
            counting_factory(Arc::new(AtomicUsize::new(0)), fake.log.clone()),
        );
        mgr.warm().expect("warm");
        let access = mgr.ensure_client().expect("ensure");
        assert!(access_is_usable(&access));

        mgr.invalidate();
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Absent
        );
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        assert!(!access_is_usable(&access));
    });
}

#[test]
fn t1_invalidate_then_ensure_client_lazy_rebuilds() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-inv-rebuild").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let spawns = Arc::new(AtomicUsize::new(0));
        let mgr = manager_with_factory(
            settings_cursor(),
            counting_factory(spawns.clone(), fake.log.clone()),
        );
        mgr.warm().expect("warm");
        let gen1 = mgr.generation_for_tests();
        assert_eq!(spawns.load(Ordering::SeqCst), 1);

        mgr.invalidate();
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);

        let access = mgr.ensure_client().expect("lazy rebuild");
        assert!(access_is_usable(&access));
        assert_eq!(spawns.load(Ordering::SeqCst), 2);
        assert_ne!(mgr.generation_for_tests(), gen1);
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
    });
}

#[test]
fn t1_lifecycle_states_cover_formal_set() {
    let states = [
        ProcessLifecycleState::Absent,
        ProcessLifecycleState::Starting,
        ProcessLifecycleState::Ready,
        ProcessLifecycleState::Invalid,
        ProcessLifecycleState::Replacing,
        ProcessLifecycleState::Stopping,
    ];
    assert_eq!(states.len(), 6);
    let _ = WarmError::Spawn;
    let _ = ShutdownError::Internal;
}
