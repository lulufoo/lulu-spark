//! Host startup coordinator — Phase-Warm wiring for `CursorAgentProcessManager`.
//!
//! Access contract: only this coordinator may call `warm()`. Non-blocking schedule
//! keeps App setup from waiting on runner spawn; warm failure never aborts Host.

use std::sync::Mutex;

use crate::config::settings;
use crate::services::agent::engine_router::{self, EngineKind, EngineRuntimeConfig};
use crate::services::agent::process_manager::{CursorAgentProcessManager, WarmError};

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

/// Production entry: read current engine/credential, then coordinate against manager.
pub fn coordinate_warm(mgr: &CursorAgentProcessManager) -> WarmCoordOutcome {
    let cfg = match settings::load() {
        Ok(s) => match engine_router::read_engine_runtime_config(&s) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[cursor-agent-process] warm skip: config error: {e}");
                return record(WarmCoordOutcome::Failed);
            }
        },
        Err(e) => {
            eprintln!("[cursor-agent-process] warm skip: settings load error: {e}");
            return record(WarmCoordOutcome::Failed);
        }
    };
    let outcome = coordinate_warm_for(&cfg, || mgr.warm());
    match outcome {
        WarmCoordOutcome::Warmed => {
            eprintln!("[cursor-agent-process] warm: Warmed");
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
    outcome
}

/// Fire-and-forget warm on a background thread so Host setup is not blocked.
pub fn schedule_cursor_runner_warm() {
    std::thread::spawn(|| {
        let _ = coordinate_warm(CursorAgentProcessManager::global());
    });
}
