//! Host startup warmup coordination.
//!
//! The coordination seam remains available for a future Agent Loop warmup
//! implementation. This release deliberately defers warmup and performs no
//! assistant initialization during startup.

use std::sync::Mutex;
use std::time::Instant;

use crate::services::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::services::agent::engine_router::EngineRuntimeConfig;

/// Observable outcome of the retained startup warmup coordination seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmCoordOutcome {
    /// Warmup is intentionally deferred until Agent Loop warmup is designed.
    Deferred,
}

static LAST_OUTCOME: Mutex<Option<WarmCoordOutcome>> = Mutex::new(None);

fn record(outcome: WarmCoordOutcome) -> WarmCoordOutcome {
    if let Ok(mut guard) = LAST_OUTCOME.lock() {
        *guard = Some(outcome);
    }
    outcome
}

/// Last coordination outcome for probes and tests.
pub fn last_warm_outcome_for_tests() -> Option<WarmCoordOutcome> {
    LAST_OUTCOME.lock().ok().and_then(|guard| *guard)
}

/// Testable coordination seam. The callback is intentionally not invoked.
pub fn coordinate_warm_for<F>(
    _cfg: &EngineRuntimeConfig,
    _warm: F,
) -> WarmCoordOutcome
where
    F: FnOnce() -> Result<(), String>,
{
    record(WarmCoordOutcome::Deferred)
}

/// Production startup entry. Agent Loop warmup is out of scope for this cut.
pub fn coordinate_warm() -> WarmCoordOutcome {
    let trace_id = TraceId::new();
    let started = Instant::now();
    let outcome = record(WarmCoordOutcome::Deferred);
    let _ = diagnostics::log(
        DiagnosticEvent::timing(
            "assistant.startup",
            "agent_loop_warm.deferred",
            &trace_id,
            started.elapsed(),
        )
        .with_static_field("outcome", "deferred"),
    );
    outcome
}

/// Fire-and-forget startup coordination; never blocks Host setup.
pub fn schedule_agent_loop_warm() {
    let _ = diagnostics::log(DiagnosticEvent::point(
        "assistant.startup",
        "agent_loop_warm.scheduled",
        &TraceId::new(),
    ));
    std::thread::spawn(|| {
        let _ = coordinate_warm();
    });
}
