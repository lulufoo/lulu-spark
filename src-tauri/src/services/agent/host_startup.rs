//! Host lifecycle coordinator for `CursorAgentProcessManager`.
//!
//! Phase-Warm: only this coordinator may call `warm()`. Non-blocking schedule
//! keeps App setup from waiting on runner spawn; warm failure never aborts Host.
//!
//! Phase-Shutdown: only App exit may call `shutdown()` via this coordinator.
//! Shutdown failure is logged and never blocks process exit.

use std::sync::Mutex;
use std::time::Instant;

use crate::config::secrets::{self, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings;
use crate::services::agent::cursor_adapter::{
    CursorError, CursorLlmEngine, CreateRequest,
};
use crate::services::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::services::agent::engine_router::{self, EngineKind, EngineRuntimeConfig};
use crate::services::agent::profile::{
    query_business_profile, BusinessProfileSnapshot, ProfileQueryError,
};
use crate::services::agent::process_manager::{
    CursorAgentProcessManager, ShutdownError, WarmError,
};
use crate::services::mcp_server_registry::SEEDED_BUSINESS_KEY;

pub const DEFAULT_TODOS_BUSINESS_ID: &str = SEEDED_BUSINESS_KEY;
pub const DEFAULT_TODOS_PREWARM_SESSION_ID: &str = "startup-prewarm-todo_task";

#[derive(Debug)]
pub enum PrewarmError {
    Profile(ProfileQueryError),
    Create(CursorError),
}

fn log_prewarm_event(event: &'static str, trace_id: &TraceId) {
    let _ = diagnostics::log(
        DiagnosticEvent::point("assistant.startup", event, trace_id)
            .with_static_field("business_id", DEFAULT_TODOS_BUSINESS_ID),
    );
}

fn prewarm_default_todos_agent_with_trace<Q>(
    engine: &CursorLlmEngine,
    query_profile: Q,
    trace_id: TraceId,
) -> Result<(), PrewarmError>
where
    Q: FnOnce(&str) -> Result<BusinessProfileSnapshot, ProfileQueryError>,
{
    let profile = match query_profile(DEFAULT_TODOS_BUSINESS_ID) {
        Ok(profile) => profile,
        Err(error) => {
            log_prewarm_event("prewarm_create_fail", &trace_id);
            return Err(PrewarmError::Profile(error));
        }
    };
    let api_key = secrets::get_secret(KEY_LLM_API_KEY_CURSOR)
        .ok()
        .flatten()
        .unwrap_or_default();
    let request = CreateRequest {
        business_id: DEFAULT_TODOS_BUSINESS_ID.to_string(),
        session_id: DEFAULT_TODOS_PREWARM_SESSION_ID.to_string(),
        profile,
        api_key,
    };
    match engine.create(&request) {
        Ok(()) => {
            log_prewarm_event("prewarm_create_ok", &trace_id);
            Ok(())
        }
        Err(error) => {
            log_prewarm_event("prewarm_create_fail", &trace_id);
            Err(PrewarmError::Create(error))
        }
    }
}

/// Startup-only default Todos prewarm. Failure is returned to the caller so
/// App boot can log it and continue; the next business request owns retry.
pub fn prewarm_default_todos_agent() -> Result<(), PrewarmError> {
    prewarm_default_todos_agent_with_trace(
        CursorLlmEngine::global(),
        query_business_profile,
        TraceId::new(),
    )
}

/// Injected prewarm seam for startup tests.
pub fn prewarm_default_todos_agent_for<Q>(
    engine: &CursorLlmEngine,
    query_profile: Q,
) -> Result<(), PrewarmError>
where
    Q: FnOnce(&str) -> Result<BusinessProfileSnapshot, ProfileQueryError>,
{
    prewarm_default_todos_agent_with_trace(engine, query_profile, TraceId::new())
}

/// Observable outcome of a Host warm coordination attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarmCoordOutcome {
    /// `warm()` invoked successfully (process + client only; no SDK Agent create).
    Warmed,
    /// Cursor configured but no usable API key — warm skipped explicitly.
    SkippedNoApiKey,
    /// Non-Cursor engine — warm not applicable.
    SkippedNonCursor,
    /// `warm()` attempted but failed; App must continue.
    Failed,
}

static LAST_OUTCOME: Mutex<Option<WarmCoordOutcome>> = Mutex::new(None);

fn record(outcome: WarmCoordOutcome) -> WarmCoordOutcome {
    if let Ok(mut guard) = LAST_OUTCOME.lock() {
        *guard = Some(outcome);
    }
    outcome
}

/// Last warm coordination outcome (tests / probes).
pub fn last_warm_outcome_for_tests() -> Option<WarmCoordOutcome> {
    LAST_OUTCOME
        .lock()
        .ok()
        .and_then(|g| *g)
}

/// Testable gate: decide from runtime config, optionally invoke `warm`.
pub fn coordinate_warm_for<F>(cfg: &EngineRuntimeConfig, warm: F) -> WarmCoordOutcome
where
    F: FnOnce() -> Result<(), WarmError>,
{
    if cfg.engine != EngineKind::Cursor {
        return record(WarmCoordOutcome::SkippedNonCursor);
    }
    let has_key = cfg
        .credential
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_key {
        return record(WarmCoordOutcome::SkippedNoApiKey);
    }
    match warm() {
        Ok(()) => record(WarmCoordOutcome::Warmed),
        Err(_) => record(WarmCoordOutcome::Failed),
    }
}

fn warm_outcome_name(outcome: WarmCoordOutcome) -> &'static str {
    match outcome {
        WarmCoordOutcome::Warmed => "warmed",
        WarmCoordOutcome::SkippedNoApiKey => "skipped_no_api_key",
        WarmCoordOutcome::SkippedNonCursor => "skipped_non_cursor",
        WarmCoordOutcome::Failed => "failed",
    }
}

fn log_warm_outcome(trace_id: &TraceId, outcome: WarmCoordOutcome, started: Instant) {
    let _ = diagnostics::log(
        DiagnosticEvent::timing(
            "assistant.startup",
            "cursor_warm.completed",
            trace_id,
            started.elapsed(),
        )
        .with_static_field("outcome", warm_outcome_name(outcome)),
    );
}

/// Production entry: read current engine/credential, then coordinate against manager.
pub fn coordinate_warm(mgr: &CursorAgentProcessManager) -> WarmCoordOutcome {
    coordinate_warm_with_trace(mgr, TraceId::new())
}

fn coordinate_warm_with_trace(
    mgr: &CursorAgentProcessManager,
    trace_id: TraceId,
) -> WarmCoordOutcome {
    let started = Instant::now();
    let cfg = match settings::load() {
        Ok(s) => match engine_router::read_engine_runtime_config(&s) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[cursor-agent-process] warm skip: config error: {e}");
                let outcome = record(WarmCoordOutcome::Failed);
                log_warm_outcome(&trace_id, outcome, started);
                return outcome;
            }
        },
        Err(e) => {
            eprintln!("[cursor-agent-process] warm skip: settings load error: {e}");
            let outcome = record(WarmCoordOutcome::Failed);
            log_warm_outcome(&trace_id, outcome, started);
            return outcome;
        }
    };
    let outcome = coordinate_warm_for(&cfg, || mgr.warm());
    match outcome {
        WarmCoordOutcome::Warmed => {
            eprintln!("[cursor-agent-process] warm: Warmed");
            if let Err(error) = prewarm_default_todos_agent_with_trace(
                CursorLlmEngine::global(),
                query_business_profile,
                trace_id.clone(),
            ) {
                eprintln!("[cursor-agent-process] default Todos prewarm failed: {error:?}");
            }
        }
        WarmCoordOutcome::SkippedNoApiKey => {
            eprintln!("[cursor-agent-process] warm: SkippedNoApiKey");
        }
        WarmCoordOutcome::SkippedNonCursor => {
            eprintln!("[cursor-agent-process] warm: SkippedNonCursor");
        }
        WarmCoordOutcome::Failed => {
            eprintln!("[cursor-agent-process] warm: Failed (App continues)");
        }
    }
    log_warm_outcome(&trace_id, outcome, started);
    outcome
}

/// Fire-and-forget warm on a background thread so Host setup is not blocked.
pub fn schedule_cursor_runner_warm() {
    let trace_id = TraceId::new();
    let _ = diagnostics::log(DiagnosticEvent::point(
        "assistant.startup",
        "cursor_warm.scheduled",
        &trace_id,
    ));
    std::thread::spawn(move || {
        let _ = coordinate_warm_with_trace(CursorAgentProcessManager::global(), trace_id);
    });
}

// ── Phase-Shutdown (App exit only) ───────────────────────────────────────────

/// Observable outcome of an App-exit shutdown coordination attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownCoordOutcome {
    /// `shutdown()` completed; client + runner reclaimed (Stopping→Absent).
    Shutdown,
    /// `shutdown()` failed; App exit must still continue (best-effort reclaim).
    Failed,
}

static LAST_SHUTDOWN_OUTCOME: Mutex<Option<ShutdownCoordOutcome>> = Mutex::new(None);

fn record_shutdown(outcome: ShutdownCoordOutcome) -> ShutdownCoordOutcome {
    if let Ok(mut guard) = LAST_SHUTDOWN_OUTCOME.lock() {
        *guard = Some(outcome);
    }
    outcome
}

/// Last shutdown coordination outcome (tests / probes).
pub fn last_shutdown_outcome_for_tests() -> Option<ShutdownCoordOutcome> {
    LAST_SHUTDOWN_OUTCOME
        .lock()
        .ok()
        .and_then(|g| *g)
}

/// Testable gate: invoke `shutdown`, map soft failure without blocking exit.
pub fn coordinate_shutdown_for<F>(shutdown: F) -> ShutdownCoordOutcome
where
    F: FnOnce() -> Result<(), ShutdownError>,
{
    match shutdown() {
        Ok(()) => record_shutdown(ShutdownCoordOutcome::Shutdown),
        Err(_) => record_shutdown(ShutdownCoordOutcome::Failed),
    }
}

/// Production entry: reclaim managed client + resident runner on App exit.
pub fn coordinate_shutdown(mgr: &CursorAgentProcessManager) -> ShutdownCoordOutcome {
    let outcome = coordinate_shutdown_for(|| mgr.shutdown());
    match outcome {
        ShutdownCoordOutcome::Shutdown => {
            eprintln!("[cursor-agent-process] shutdown: Shutdown");
        }
        ShutdownCoordOutcome::Failed => {
            eprintln!(
                "[cursor-agent-process] shutdown: Failed (App exit continues; best-effort reclaim)"
            );
        }
    }
    outcome
}

/// App exit hook: call from `RunEvent::Exit` only. Never panics; never blocks exit.
pub fn on_app_exit_shutdown() {
    let _ = coordinate_shutdown(CursorAgentProcessManager::global());
}
