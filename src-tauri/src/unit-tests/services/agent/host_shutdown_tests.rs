//! T-Shutdown / Phase-Shutdown: App exit wires `CursorAgentProcessManager::shutdown()`.
//!
//! Access 合同: only App exit path may call `shutdown()`. Engine/business switch must
//! not reclaim the resident runner; shutdown failure is soft (log, never block exit).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings, LlmSettings};
use crate::services::agent::cursor_adapter::{
    CursorError, CursorRunnerClient, FakeCursorRunnerClient,
};
use crate::services::agent::engine_router::{self, EngineRuntimeConfig};
use crate::services::agent::host_startup::{self, ShutdownCoordOutcome};
use crate::services::agent::process_manager::{
    self, CursorAgentProcessManager, ProcessLifecycleState, ShutdownError,
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

// ── Module / Exit wiring surface ─────────────────────────────────────────────

#[test]
fn t8_host_shutdown_coordinator_surface_exists() {
    let _ = std::any::type_name::<ShutdownCoordOutcome>();
    let src = include_str!("../../../services/agent/host_startup.rs");
    assert!(
        src.contains("fn coordinate_shutdown"),
        "Host exit coordinator must expose coordinate_shutdown"
    );
    assert!(
        src.contains("fn coordinate_shutdown_for"),
        "testable gate coordinate_shutdown_for required"
    );
    assert!(
        src.contains("fn on_app_exit_shutdown"),
        "App exit must expose on_app_exit_shutdown entry"
    );
    assert!(
        src.contains(".shutdown()") || src.contains("shutdown()"),
        "coordinator must go through manager.shutdown()"
    );
    assert!(
        !src.contains("ProcessCursorRunnerClient::spawn")
            && !src.contains("ProcessCursorRunnerClient::new"),
        "shutdown coordinator must not construct ProcessCursorRunnerClient"
    );
}

#[test]
fn t8_lib_exit_wires_on_app_exit_shutdown() {
    let lib = include_str!("../../../lib.rs");
    assert!(
        lib.contains("on_app_exit_shutdown"),
        "App exit path must call on_app_exit_shutdown (Phase-Shutdown)"
    );
    let exit_idx = lib
        .find("RunEvent::Exit")
        .expect("tauri RunEvent::Exit handler required");
    let exit_tail = &lib[exit_idx..];
    // Bound the Exit arm roughly to the next major close / end of run closure.
    let exit_end = exit_tail
        .find("\n        });")
        .or_else(|| exit_tail.find("\n    });"))
        .unwrap_or(exit_tail.len().min(800));
    let exit_body = &exit_tail[..exit_end];
    assert!(
        exit_body.contains("on_app_exit_shutdown"),
        "on_app_exit_shutdown must live inside RunEvent::Exit handler"
    );
    assert!(
        !exit_body.contains("ProcessCursorRunnerClient::spawn"),
        "exit must not bypass manager to spawn ProcessCursorRunnerClient"
    );
}

#[test]
fn t8_access_contract_only_app_exit_calls_shutdown() {
    // Access 合同: only App exit coordinator may call manager.shutdown().
    for (label, src) in [
        ("loop.rs", include_str!("../../../services/agent/loop.rs")),
        ("llm.rs", include_str!("../../../services/agent/llm.rs")),
        ("session.rs", include_str!("../../../services/agent/session.rs")),
        (
            "engine_router.rs",
            include_str!("../../../services/agent/engine_router.rs"),
        ),
        (
            "runtime.rs",
            include_str!("../../../services/agent/runtime.rs"),
        ),
        (
            "cursor_adapter.rs",
            include_str!("../../../services/agent/cursor_adapter.rs"),
        ),
    ] {
        assert!(
            !src.contains(".shutdown(") && !src.contains("::shutdown("),
            "{label} must not call manager.shutdown() (Access 合同: only App exit)"
        );
    }
    let host = include_str!("../../../services/agent/host_startup.rs");
    assert!(
        host.contains(".shutdown(") || host.contains("shutdown()"),
        "App exit coordinator (host_startup) must call shutdown()"
    );
    // Warm path must not also shut down (resident until App exit).
    let warm_only = host
        .lines()
        .filter(|l| l.contains("warm") || l.contains("Warm"))
        .collect::<Vec<_>>();
    for line in warm_only {
        assert!(
            !line.contains("shutdown"),
            "warm path must not invoke shutdown: {line}"
        );
    }
}

// ── Happy: exit reclaim Stopping→Absent, no residual managed runner ──────────

#[test]
fn t8_coordinate_shutdown_reclaims_resident_runner_to_absent() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t8-exit").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let spawns = Arc::new(AtomicUsize::new(0));
        let shutdown_calls = Arc::new(AtomicUsize::new(0));
        let mgr =
            manager_with_factory(settings_cursor(), counting_factory(spawns.clone(), log.clone()));
        mgr.warm().expect("warm");
        assert_eq!(mgr.managed_runner_count_for_tests(), 1);
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );

        let shutdown_calls_cb = shutdown_calls.clone();
        let outcome = host_startup::coordinate_shutdown_for(|| {
            shutdown_calls_cb.fetch_add(1, Ordering::SeqCst);
            mgr.shutdown()
        });

        assert_eq!(outcome, ShutdownCoordOutcome::Shutdown);
        assert_eq!(shutdown_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Absent,
            "shutdown must end in Absent (via Stopping)"
        );
        assert_eq!(
            mgr.managed_runner_count_for_tests(),
            0,
            "exit must leave no residual managed runner"
        );
        assert!(
            log.lock().unwrap().iter().any(|e| e.method == "close")
                || fake.force_killed.load(Ordering::SeqCst),
            "shutdown must reclaim client/process"
        );
        assert_eq!(
            host_startup::last_shutdown_outcome_for_tests(),
            Some(ShutdownCoordOutcome::Shutdown)
        );
        // Resident spawn count stays at warm's one — no re-spawn on exit.
        assert_eq!(spawns.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn t8_coordinate_shutdown_production_entry_reclaims_global() {
    with_sandbox(|| {
        // Do not warm() the production global (real Node spawn). Exercise exit
        // coordinator against Absent global — shutdown must still soft-succeed.
        settings::save(&settings_cursor()).expect("save");
        let mgr = CursorAgentProcessManager::global();
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        let outcome = host_startup::coordinate_shutdown(mgr);
        assert_eq!(outcome, ShutdownCoordOutcome::Shutdown);
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Absent
        );
        assert_eq!(
            host_startup::last_shutdown_outcome_for_tests(),
            Some(ShutdownCoordOutcome::Shutdown)
        );
    });
}

// ── Boundary: engine / business switch must NOT shut down resident runner ────

#[test]
fn t8_engine_switch_and_business_paths_do_not_call_shutdown() {
    // leaf-process-manager-AR / leaf-engine-config-switch-AR: resident until App exit.
    for (label, src) in [
        (
            "engine_router.rs",
            include_str!("../../../services/agent/engine_router.rs"),
        ),
        (
            "runtime.rs",
            include_str!("../../../services/agent/runtime.rs"),
        ),
        ("loop.rs", include_str!("../../../services/agent/loop.rs")),
        (
            "cursor_adapter.rs",
            include_str!("../../../services/agent/cursor_adapter.rs"),
        ),
    ] {
        assert!(
            !src.contains("coordinate_shutdown")
                && !src.contains("on_app_exit_shutdown")
                && !src.contains(".shutdown("),
            "{label} must not shut down resident runner on engine/business switch"
        );
    }
}

#[test]
fn t8_switching_to_non_cursor_keeps_resident_runner_until_exit() {
    with_sandbox(|| {
        secrets::set_secret(KEY_LLM_API_KEY_CURSOR, "sk-t8-switch").expect("key");
        let fake = FakeCursorRunnerClient::new();
        let log = fake.log.clone();
        let settings_cell = Arc::new(Mutex::new(settings_cursor()));
        let settings_for_mgr = settings_cell.clone();
        let mgr = CursorAgentProcessManager::with_deps_for_tests(
            move || {
                let s = settings_for_mgr
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                Ok(cfg_from(&s))
            },
            counting_factory(Arc::new(AtomicUsize::new(0)), log),
        );
        mgr.warm().expect("warm while Cursor");
        assert_eq!(mgr.managed_runner_count_for_tests(), 1);

        // Simulate runtime switch to non-Cursor — must NOT reclaim runner.
        *settings_cell.lock().unwrap() = settings_host();
        assert_eq!(
            mgr.managed_runner_count_for_tests(),
            1,
            "switch to non-Cursor must keep resident runner until App exit"
        );
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Ready
        );
        assert!(
            !fake.force_killed.load(Ordering::SeqCst),
            "engine switch must not force_kill resident runner"
        );

        // Only App exit coordinator reclaims.
        let outcome = host_startup::coordinate_shutdown_for(|| mgr.shutdown());
        assert_eq!(outcome, ShutdownCoordOutcome::Shutdown);
        assert_eq!(mgr.managed_runner_count_for_tests(), 0);
        assert_eq!(
            mgr.lifecycle_state_for_tests(),
            ProcessLifecycleState::Absent
        );
    });
}

// ── Failure: shutdown Err must not block App exit ────────────────────────────

#[test]
fn t8_shutdown_failure_is_soft_app_exit_continues() {
    with_sandbox(|| {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_cb = calls.clone();
        let outcome = host_startup::coordinate_shutdown_for(|| {
            calls_cb.fetch_add(1, Ordering::SeqCst);
            Err(ShutdownError::Internal)
        });
        assert_eq!(outcome, ShutdownCoordOutcome::Failed);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            host_startup::last_shutdown_outcome_for_tests(),
            Some(ShutdownCoordOutcome::Failed)
        );
        // Soft contract: returning Failed (not panic) means exit path can continue.
    });
}

#[test]
fn t8_on_app_exit_shutdown_does_not_panic_on_failure() {
    with_sandbox(|| {
        // Global Absent + soft path — must never panic / block.
        host_startup::on_app_exit_shutdown();
        let outcome = host_startup::last_shutdown_outcome_for_tests();
        assert!(
            matches!(
                outcome,
                Some(ShutdownCoordOutcome::Shutdown) | Some(ShutdownCoordOutcome::Failed)
            ),
            "on_app_exit_shutdown must record outcome, got {outcome:?}"
        );
    });
}

#[test]
fn t8_shutdown_outside_communication_failure_replay_semantics() {
    // Phase-Shutdown / T-Errors: shutdown is not in the communication-invalid set;
    // exit reclaim must not be mapped as recoverable-failure replay.
    let host = include_str!("../../../services/agent/host_startup.rs");
    assert!(
        !host.contains("RecoverableFailure") && !host.contains("replay"),
        "shutdown coordinator must not map exit reclaim into recoverable-failure replay"
    );
    let _ = ShutdownError::Internal;
    let _ = std::any::type_name::<ShutdownCoordOutcome>();
}
