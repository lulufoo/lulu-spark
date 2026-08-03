//! Per-session independent working directory lifecycle (Cursor Local path).
//!
//! Creates a unique cwd per session for Agent SDK Local `local.cwd`, and
//! exposes cleanup hooks for session end. Create failure never silently shares
//! a global/shared cwd; cleanup failure retries once then retains + warns.
//!
//! ## A1 (L3 Must Close Before P4 / F-46–F-47)
//!
//! Confirmed (not silently narrowed): Cursor Agent Local is used with the
//! combination of per-session independent `local.cwd` **and** injected
//! `mcpServers` on the same `Agent.create` params (see `cursor_adapter`).
//! This module owns cwd lifecycle; `cursor_adapter::run_turn_for_session`
//! wires the allocated path into `local.cwd` alongside mcpServers.
//!
//! Failure paths here must not fall back to process-local business tool dispatch
//! (L09-I #7).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use crate::services::agent::session;

/// A1 confirmed: per-session `local.cwd` + injected `mcpServers` combo is the
/// Cursor Local delivery strategy (wired via `cursor_adapter::run_turn_for_session`).
pub const CURSOR_LOCAL_A1_CWD_MCPSERVERS_COMBO: bool = true;

/// Default cleanup strategy for session end.
pub const DEFAULT_CLEANUP_STRATEGY: CleanupStrategy = CleanupStrategy::ImmediateDelete;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupStrategy {
    /// Delete the session cwd directory immediately on cleanup.
    ImmediateDelete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CwdError {
    InvalidSessionId,
    CreateFailed(String),
    /// Cleanup failed after retry — directory retained; message is the warning.
    CleanupRetained(String),
}

impl std::fmt::Display for CwdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CwdError::InvalidSessionId => write!(f, "invalid session_id for cwd"),
            CwdError::CreateFailed(msg) => write!(f, "session cwd create failed: {msg}"),
            CwdError::CleanupRetained(msg) => {
                write!(f, "session cwd cleanup retained after retry: {msg}")
            }
        }
    }
}

impl std::error::Error for CwdError {}

fn allocated() -> &'static Mutex<HashMap<String, PathBuf>> {
    static ALLOCATED: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
    ALLOCATED.get_or_init(|| Mutex::new(HashMap::new()))
}

fn validate_session_id(session_id: &str) -> Result<&str, CwdError> {
    let id = session_id.trim();
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(CwdError::InvalidSessionId);
    }
    Ok(id)
}

/// Primary root: `{cache_dir}/agent/session-cwds`.
pub fn primary_session_cwds_root() -> Result<PathBuf, CwdError> {
    let agent = session::agent_dir().map_err(|e| CwdError::CreateFailed(e))?;
    Ok(agent.join("session-cwds"))
}

/// Alternate isolation root (still per-session): system temp under workbench prefix.
/// Used only when primary create fails — never a shared/global cwd fallback.
pub fn alternate_session_cwds_root() -> PathBuf {
    std::env::temp_dir().join("lulu-workbench-session-cwds")
}

fn cwd_path_under(root: &Path, session_id: &str) -> PathBuf {
    root.join(session_id)
}

fn try_create_dir(path: &Path) -> Result<(), String> {
    if cfg!(test) && force_create_fail() {
        return Err("forced create failure (test)".into());
    }
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    if !path.is_dir() {
        return Err(format!("path is not a directory after create: {path:?}"));
    }
    Ok(())
}

/// Create an independent working directory for `session_id`.
///
/// Strategy: primary under cache; on failure try alternate isolation root
/// (still per-session unique). Never silently share a global cwd.
pub fn create_session_cwd(session_id: &str) -> Result<PathBuf, CwdError> {
    let id = validate_session_id(session_id)?.to_string();

    if let Some(existing) = session_cwd_for(&id) {
        if existing.is_dir() {
            return Ok(existing);
        }
    }

    set_alternate_attempted(false);

    let primary_root = primary_session_cwds_root()?;
    let primary_path = cwd_path_under(&primary_root, &id);
    match try_create_dir(&primary_path) {
        Ok(()) => {
            register(&id, primary_path.clone());
            return Ok(primary_path);
        }
        Err(primary_err) => {
            // Alternate isolation strategy (still per-session) — not shared cwd.
            set_alternate_attempted(true);
            let alt_root = alternate_session_cwds_root();
            let alt_path = cwd_path_under(&alt_root, &id);
            match try_create_dir(&alt_path) {
                Ok(()) => {
                    register(&id, alt_path.clone());
                    return Ok(alt_path);
                }
                Err(alt_err) => {
                    return Err(CwdError::CreateFailed(format!(
                        "primary={primary_err}; alternate={alt_err}"
                    )));
                }
            }
        }
    }
}

/// Observable allocated cwd for a session (if any).
pub fn session_cwd_for(session_id: &str) -> Option<PathBuf> {
    let id = session_id.trim();
    if id.is_empty() {
        return None;
    }
    allocated()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(id)
        .cloned()
}

fn register(session_id: &str, path: PathBuf) {
    allocated()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(session_id.to_string(), path);
}

fn unregister(session_id: &str) {
    allocated()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(session_id);
}

fn remove_dir_once(path: &Path) -> Result<(), String> {
    if cfg!(test) && force_cleanup_fail() {
        return Err("forced cleanup failure (test)".into());
    }
    if !path.exists() {
        return Ok(());
    }
    fs::remove_dir_all(path).map_err(|e| e.to_string())
}

/// Clean up the session working directory (default: immediate delete).
///
/// On failure: retry once; if still failing, retain the directory and return
/// [`CwdError::CleanupRetained`] (never silently ignore). Also logs a warning.
pub fn cleanup_session_cwd(session_id: &str) -> Result<(), CwdError> {
    cleanup_session_cwd_with_strategy(session_id, DEFAULT_CLEANUP_STRATEGY)
}

pub fn cleanup_session_cwd_with_strategy(
    session_id: &str,
    strategy: CleanupStrategy,
) -> Result<(), CwdError> {
    let id = validate_session_id(session_id)?.to_string();
    let path = match session_cwd_for(&id) {
        Some(p) => p,
        None => {
            // Best-effort: try primary path even if not registered.
            let root = primary_session_cwds_root().unwrap_or_else(|_| alternate_session_cwds_root());
            cwd_path_under(&root, &id)
        }
    };

    match strategy {
        CleanupStrategy::ImmediateDelete => {
            let first = remove_dir_once(&path);
            if first.is_ok() {
                unregister(&id);
                return Ok(());
            }
            // Retry once.
            let second = remove_dir_once(&path);
            if second.is_ok() {
                unregister(&id);
                return Ok(());
            }
            let msg = format!(
                "cleanup failed after retry for {id}: first={:?}; second={:?}",
                first.err(),
                second.err()
            );
            let _ = session::log_agent_error(&format!("session_cwd: {msg}"));
            // Keep registration so the retained path stays observable.
            Err(CwdError::CleanupRetained(msg))
        }
    }
}

// ── test hooks ──────────────────────────────────────────────────────────────

#[cfg(test)]
thread_local! {
    static FORCE_CREATE_FAIL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FORCE_CLEANUP_FAIL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static ALTERNATE_ATTEMPTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(not(test))]
fn force_create_fail() -> bool {
    false
}

#[cfg(test)]
fn force_create_fail() -> bool {
    FORCE_CREATE_FAIL.with(|c| c.get())
}

#[cfg(not(test))]
fn force_cleanup_fail() -> bool {
    false
}

#[cfg(test)]
fn force_cleanup_fail() -> bool {
    FORCE_CLEANUP_FAIL.with(|c| c.get())
}

fn set_alternate_attempted(v: bool) {
    #[cfg(test)]
    ALTERNATE_ATTEMPTED.with(|c| c.set(v));
    #[cfg(not(test))]
    let _ = v;
}

/// Reset in-memory allocation map and test hooks (unit tests only).
pub fn reset_for_tests() {
    allocated()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    #[cfg(test)]
    {
        FORCE_CREATE_FAIL.with(|c| c.set(false));
        FORCE_CLEANUP_FAIL.with(|c| c.set(false));
        ALTERNATE_ATTEMPTED.with(|c| c.set(false));
    }
}

pub fn force_create_fail_for_tests(fail: bool) {
    #[cfg(test)]
    FORCE_CREATE_FAIL.with(|c| c.set(fail));
    #[cfg(not(test))]
    let _ = fail;
}

pub fn force_cleanup_fail_for_tests(fail: bool) {
    #[cfg(test)]
    FORCE_CLEANUP_FAIL.with(|c| c.set(fail));
    #[cfg(not(test))]
    let _ = fail;
}

pub fn alternate_isolation_attempted_for_tests() -> bool {
    #[cfg(test)]
    {
        ALTERNATE_ATTEMPTED.with(|c| c.get())
    }
    #[cfg(not(test))]
    {
        false
    }
}
