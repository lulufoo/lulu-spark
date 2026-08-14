//! Startup warmup remains as a deferred Agent Loop seam.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::services::agent::engine_router::{EngineKind, EngineRuntimeConfig};
use crate::services::agent::host_startup::{self, WarmCoordOutcome};

fn host_runtime_config() -> EngineRuntimeConfig {
    EngineRuntimeConfig {
        engine: EngineKind::Host,
        model: "glm-4".into(),
        credential: Some("sk-test".into()),
    }
}

#[test]
fn warmup_coordinator_is_retained_without_legacy_shutdown_wiring() {
    let source = include_str!("../../../services/agent/host_startup.rs");
    assert!(source.contains("fn coordinate_warm"));
    assert!(source.contains("fn coordinate_warm_for"));
    assert!(source.contains("fn schedule_agent_loop_warm"));
    assert!(!source.contains("Cursor"));
    assert!(!source.contains("cursor"));
    assert!(!source.contains("ProcessManager"));
    assert!(!source.contains("coordinate_shutdown"));
    assert!(!source.contains("on_app_exit_shutdown"));

    let lib = include_str!("../../../lib.rs");
    assert!(lib.contains("schedule_agent_loop_warm"));
    assert!(!lib.contains("schedule_cursor_runner_warm"));
    assert!(!lib.contains("on_app_exit_shutdown"));
}

#[test]
fn coordinate_warm_defers_and_never_invokes_the_old_callback() {
    let calls = AtomicUsize::new(0);
    let outcome = host_startup::coordinate_warm_for(&host_runtime_config(), || {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });

    assert_eq!(outcome, WarmCoordOutcome::Deferred);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        host_startup::last_warm_outcome_for_tests(),
        Some(WarmCoordOutcome::Deferred)
    );
}

#[test]
fn production_warmup_is_a_nonblocking_deferred_noop() {
    let outcome = host_startup::coordinate_warm();
    assert_eq!(outcome, WarmCoordOutcome::Deferred);
}

#[test]
fn scheduled_warmup_returns_without_waiting_for_agent_initialization() {
    host_startup::schedule_agent_loop_warm();
}
