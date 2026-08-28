//! Binding Contract: set / reset / query / execute, MCP ticket, generation.

use std::sync::atomic::Ordering;

use serde_json::{json, Value};

use crate::services::agent::path_fence::PathFence;
use crate::services::agent::session::{self, BindingStateSummary, SetError};
use crate::services::mcp_oauth::{issue_for_slot, Slot};
use crate::services::mcp_server_registry::{self, McpServerConfig, McpServerLookupError};
use crate::services::workbench_path_fence;

use super::flights::{emit_lifecycle, emit_shell_binding_changed, request_in_flight_cancel, runtime};
use super::types::{ChatTurnResult, ExecError, ExecuteOutcome};

/// Set Binding after validation. Failure returns `set_invalid` and leaves state unchanged.
/// Legal Set on bound atomically replaces and invalidates the previous generation.
/// Any successful Set (first or replace) clears `current_session_id` in the same critical
/// section as generation advance (session cut, clear-first).
/// Cancels in-flight chat (busy) symmetrically; in-flight execute is interrupted via
/// generation invalidation (distinguishable stale reject preserved).
/// Emits onBound; replace also emits onUnbound → onBound (D1).
/// Illegal Set emits onError(set_invalid) and does not emit onBound.
///
/// Typed/internal Set (no business key) clears any previously loaded MCP config.
/// Key-only public Set goes through `try_set_binding_json` which loads MCP first.
pub fn set_binding(binding: session::Binding) -> Result<(), SetError> {
    set_binding_with_mcp(binding, None, None, None)
}

fn set_binding_with_mcp(
    binding: session::Binding,
    loaded_mcp: Option<McpServerConfig>,
    business_id: Option<String>,
    loaded_path_fence: Option<PathFence>,
) -> Result<(), SetError> {
    if let Err(e) = session::validate_binding(&binding) {
        emit_lifecycle("onError", Some("set_invalid"));
        return Err(e);
    }
    let (was_bound, previous_business_id, generation, mcp_url) = {
        // Lock order: Runtime (orchestration) → AIAssistantSession (live context).
        let rt = runtime().lock().unwrap();
        session::with_live_mut(|live| {
            let was = live.current_binding.is_some();
            let previous_business_id = live.current_business_id.clone();
            // Chat cancel on cut; do not set execute_cancelled here so replace mid-execute
            // still returns rejected_stale_generation (generation advance is the execute interrupt).
            if rt.busy || !rt.flights.is_empty() {
                live.chat_cancelled = true;
                for flight in rt.flights.values() {
                    flight.cancel.store(true, Ordering::Relaxed);
                }
            }
            live.generation_seq = live.generation_seq.saturating_add(1);
            live.current_generation = Some(live.generation_seq);
            live.current_binding = Some(binding);
            live.current_business_id = business_id.clone();
            live.loaded_mcp_server = loaded_mcp;
            live.loaded_path_fence = loaded_path_fence;
            // Session cut: clear live id with generation update (no new-gen + old-session window).
            live.current_session_id = None;
            let generation = live.current_generation;
            let mcp_url = live
                .loaded_mcp_server
                .as_ref()
                .map(|config| config.http_transport().url.clone());
            (was, previous_business_id, generation, mcp_url)
        })
    };
    eprintln!(
        "[DEBUG-binding-transition] op=set previous_business_id={previous_business_id:?} \
         business_id={business_id:?} generation={generation:?} mcp_url={mcp_url:?}"
    );
    if was_bound {
        emit_lifecycle("onUnbound", None);
    }
    emit_lifecycle("onBound", None);
    // Shell sync: Bound / new Bound — shell must discard cached sessionId and follow query_binding.
    emit_shell_binding_changed();
    Ok(())
}

/// JSON Set entry: key-only public contract. Looks up Host MCP registry and loads
/// session capability context. Rejects legacy tools/prompt/callbacks payload.
pub fn try_set_binding_json(v: &Value) -> Result<(), SetError> {
    let parsed = session::binding_from_json(v).map_err(|e| {
        emit_lifecycle("onError", Some(e.as_code()));
        e
    })?;
    let key = session::binding_business_key(&parsed).ok_or_else(|| {
        emit_lifecycle("onError", Some("set_invalid"));
        SetError::set_invalid()
    })?;
    let config = match mcp_server_registry::lookup(&key) {
        Ok(cfg) => cfg,
        Err(McpServerLookupError::NotFound) => {
            emit_lifecycle("onError", Some("unknown_key"));
            return Err(SetError::unknown_key());
        }
        Err(McpServerLookupError::InvalidKey) => {
            emit_lifecycle("onError", Some("set_invalid"));
            return Err(SetError::set_invalid());
        }
    };
    let config = inject_workbench_ticket(key.as_str(), config)?;
    let fence = workbench_path_fence::expand_for_business_key(key.as_str());
    // L1+L2: public key-only Set must not feed business tool handles to the
    // Agent Loop. The business key is already resolved into loaded_mcp_server;
    // Binding.tools stays an empty interface slot. File tools are Host-owned
    // and gated by the fence attached here, not by caller-supplied paths.
    let binding = session::Binding {
        tools: json!([]),
        prompt: json!(config.capability_description.clone()),
        callbacks: parsed.callbacks,
    };
    set_binding_with_mcp(binding, Some(config), Some(key), fence)
}

/// After workbench lookup: reuse a Live ticket or issue one, then write
/// Authorization onto this session's transport copy. Registry seed is unchanged.
/// Keychain failure returns `set_invalid` and must not reach `set_binding_with_mcp`.
fn inject_workbench_ticket(key: &str, mut config: McpServerConfig) -> Result<McpServerConfig, SetError> {
    if key != mcp_server_registry::SEEDED_BUSINESS_KEY {
        return Ok(config);
    }
    let handle = match issue_for_slot(Slot::Workbench) {
        Ok(handle) => handle,
        Err(_) => {
            emit_lifecycle("onError", Some("set_invalid"));
            return Err(SetError::set_invalid());
        }
    };
    config.http_transport.headers.insert(
        "Authorization".to_string(),
        format!("Bearer {}", handle.as_str()),
    );
    Ok(config)
}

/// The decision-level `McpServerConfig` shape is the shared read form for the
/// Host Agent Loop. Field-level transport schema remains deferred.
pub const SESSION_CAPABILITY_READ_FACE_REGISTRY_SHAPE: bool = true;

/// Must Close Before T4 — A2 confirmed (not narrowed): Host `mcp_server_registry`
/// is the sole lookup source; this face only exposes config already loaded by
/// key-only Set from that table.
pub const SESSION_CAPABILITY_READ_FACE_A2_HOST_REGISTRY_SOLE_LOOKUP: bool = true;

/// Typed/internal bindings without an MCP capability still complete text-only facade turns.
pub const HOST_EMPTY_TOOLS_FACADE_USABLE: bool = true;

/// Read-only session capability consumption face for the Host Agent Loop.
/// Returns a detached clone of the MCP Server config loaded by key-only Binding
/// Set. No write path — only Set/Reset lifecycle may change the loaded value.
pub fn session_capability_mcp_config() -> Option<McpServerConfig> {
    session::live_context_owner().loaded_mcp_server()
}

/// Host file-tool fence attached at public key-only Set. Typed/internal Set
/// leaves this empty. Scratch root is resolved per turn from `scratch_parent`.
pub fn loaded_path_fence() -> Option<PathFence> {
    session::live_context_owner().loaded_path_fence()
}

/// Observability alias for the session capability read face.
pub fn loaded_mcp_server() -> Option<McpServerConfig> {
    session_capability_mcp_config()
}

/// Reset unloads this session's Binding, MCP config, and ticket header only.
/// MUST NOT call `revoke_for_slot`; the workbench ledger ticket stays Live.
pub const RESET_UNLOADS_SESSION_ONLY: bool = true;

/// Reset: discard current Binding → unbound. Idempotent when already unbound.
/// Clears `current_session_id` (session cut, clear-first); does not wipe disk turns.
/// Unloads session capability MCP config and ticket header with the Binding.
/// Does not revoke the workbench ledger ticket; next Set reuses the Live handle.
/// Bound→unbound emits onUnbound; symmetrically cancels in-flight execute and chat.
pub fn reset_binding() -> Result<(), ()> {
    let (was_bound, previous_business_id, previous_generation) = {
        let rt = runtime().lock().unwrap();
        session::with_live_mut(|live| {
            let was = live.current_binding.is_some();
            let previous_business_id = live.current_business_id.clone();
            let previous_generation = live.current_generation;
            request_in_flight_cancel(&rt, live);
            live.current_binding = None;
            live.current_business_id = None;
            live.loaded_mcp_server = None;
            live.loaded_path_fence = None;
            live.current_generation = None;
            live.current_session_id = None;
            (was, previous_business_id, previous_generation)
        })
    };
    eprintln!(
        "[DEBUG-binding-transition] op=reset was_bound={was_bound} \
         previous_business_id={previous_business_id:?} previous_generation={previous_generation:?}"
    );
    if was_bound {
        emit_lifecycle("onUnbound", None);
        // Shell sync: Bound→Unbound (defensive_unbound shares this path — no core-only bypass).
        emit_shell_binding_changed();
    }
    Ok(())
}

/// Close the Host-owned Agent slot by the live session identity, then reset
/// the current Binding. Close runs before Reset clears the live context.
/// Same as `reset_binding`: unloads session only; does not revoke the ledger ticket.
pub fn reset_binding_with_close<Close>(mut close: Close) -> Result<(), String>
where
    Close: FnMut(&str) -> Result<(), String>,
{
    let session_id = session::live_context_owner().current_session_id();
    if let Some(session_id) = session_id {
        close(&session_id)?;
    }
    reset_binding().map_err(|_| "reset_binding_failed".to_string())?;
    Ok(())
}

/// Confirmed Host hook path for defensive cut (P2 Must Close Before).
/// Call when leave/unmount was observed as missed while still bound.
pub const DEFENSIVE_CUT_HOOK_PATH: &str =
    "src-tauri/src/services/agent/loop/binding.rs::defensive_unbound";

/// Explicit leave→Reset chain that defensive cut backs (does not replace).
pub const DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN: &[&str] = &[
    "frontend/src/todo-task/index.js::dispose",
    "frontend/src/todo-task/lifecycle.js::onTodosPageLeave",
    "frontend/src/todo-task/binding.js::resetTodosBinding",
    "src-tauri/src/services/agent/loop/binding.rs::reset_binding",
];

/// T6 layered acceptance L0 markers (regression): unbound reject / Reset idempotent / mid-Reset cancel.
pub const LAYERED_ACCEPTANCE_L0: &[&str] = &[
    "unbound_reject_execute",
    "reset_idempotent",
    "mid_reset_cancel",
];

/// T6 layered acceptance L1 markers (must-add): session/generation/re-Set cuts.
pub const LAYERED_ACCEPTANCE_L1: &[&str] = &[
    "old_session_not_executable_after_reset_or_replace_set",
    "stale_generation_reject_continue",
    "re_set_without_old_turns",
    "any_successful_set_clears_pre_set_session",
];

/// T6 layered acceptance L2 markers (must-add): missed dispose/Reset → defensive cut.
pub const LAYERED_ACCEPTANCE_L2: &[&str] =
    &["missed_dispose_or_reset_defensive_cut_not_executable"];

/// Todos normal leave still uses explicit Reset as primary (defensive cut does not replace).
pub const TODOS_EXPLICIT_LEAVE_RESET_PRIMARY: &[&str] = DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN;

/// Host defensive cut: same observable semantics as `reset_binding` (unbound + gen
/// invalidate + session clear + symmetric in-flight cancel).
/// Backs missed leave on `DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN`; does not replace explicit Reset.
/// `shell_close_core` must not call this.
pub fn defensive_unbound() -> Result<(), ()> {
    reset_binding()
}

/// Binding Contract execute: only when bound; applies current Binding tools/prompt as sole config.
pub fn execute_binding() -> Result<ExecuteOutcome, ExecError> {
    execute_binding_during(|| {})
}

/// Like `execute_binding`, but invokes `mid` while the execute is marked in-flight
/// (for Strategy A mid-execute Reset observation).
pub fn execute_binding_during<F: FnOnce()>(mid: F) -> Result<ExecuteOutcome, ExecError> {
    let (snapshot, generation) = {
        let mut rt = runtime().lock().unwrap();
        let prepared = session::with_live_mut(|live| {
            let Some(binding) = live.current_binding.clone() else {
                return Err("unbound");
            };
            let Some(generation) = live.current_generation else {
                return Err("stale");
            };
            live.execute_cancelled = false;
            Ok((binding, generation))
        });
        match prepared {
            Ok(pair) => {
                rt.executing = true;
                pair
            }
            Err("unbound") => {
                drop(rt);
                emit_lifecycle("onError", Some("rejected_unbound"));
                return Err(ExecError::rejected_unbound());
            }
            Err(_) => {
                drop(rt);
                return Err(exec_err_stale_generation());
            }
        }
    };

    mid();

    let cancelled = {
        let mut rt = runtime().lock().unwrap();
        let cancelled = session::with_live_mut(|live| live.execute_cancelled);
        rt.executing = false;
        cancelled
    };

    if cancelled {
        emit_lifecycle("onError", Some("reset_cancelled"));
        return Err(ExecError::reset_cancelled());
    }
    if !is_binding_generation_current(generation) {
        return Err(exec_err_stale_generation());
    }

    Ok(ExecuteOutcome {
        applied_tools: snapshot.tools,
        applied_prompt: snapshot.prompt,
    })
}

/// Read-only query: state∈{unbound,bound}; bound includes generation only (no tools/prompt/business IDs).
pub fn query_binding() -> BindingStateSummary {
    query_binding_snapshot().0
}

/// Atomic snapshot of query summary + current Binding slot count (0 or 1).
pub fn query_binding_snapshot() -> (BindingStateSummary, usize) {
    session::with_live_mut(|live| match live.current_binding.as_ref() {
        Some(_) => (
            BindingStateSummary {
                state: "bound",
                generation: live.current_generation,
            },
            1,
        ),
        None => (
            BindingStateSummary {
                state: "unbound",
                generation: None,
            },
            0,
        ),
    })
}

/// Current binding gate: `unbound` | `bound` (generic Binding present).
pub fn binding_state() -> &'static str {
    query_binding().state
}

/// How many current Binding slots are held (0 or 1). Used to prove single-binding serialization.
pub fn current_binding_slot_count() -> usize {
    query_binding_snapshot().1
}

/// Whether `generation` is still the live current Binding (false after Reset or replace).
pub fn is_binding_generation_current(generation: u64) -> bool {
    session::live_context_owner().current_generation() == Some(generation)
}

fn exec_err_stale_generation() -> ExecError {
    emit_lifecycle("onError", Some("rejected_stale_generation"));
    ExecError::rejected_stale_generation()
}

/// Atomic snapshot of current Binding + live generation (None if unbound or gen missing).
pub(super) fn current_binding_generation_snapshot() -> Option<(session::Binding, u64)> {
    session::with_live_mut(|live| match (live.current_binding.clone(), live.current_generation) {
        (Some(binding), Some(generation)) => Some((binding, generation)),
        _ => None,
    })
}

/// Binding Contract ops only (L03-SC / L08-AR). Present/Open are shell surface, not ops.
pub const BINDING_CONTRACT_OPS: &[&str] = &["set", "reset", "query", "execute", "callbacks"];

/// Stated Binding Contract ops list (excludes Present / Open).
pub fn binding_contract_ops() -> &'static [&'static str] {
    BINDING_CONTRACT_OPS
}

pub(super) fn current_binding_snapshot() -> Option<session::Binding> {
    session::live_context_owner().current_binding()
}

/// Execute the business request path from the current Binding:
/// ensure-create first, then turn. The callbacks receive the live session id
/// as the Agent isolation key; Binding-derived identity stays on live to
/// select capability and is not passed into these callbacks.
pub fn ensure_create_then_turn<Create, Turn>(
    prompt: &str,
    create: Create,
    turn: Turn,
) -> Result<ChatTurnResult, String>
where
    Create: FnMut(&str) -> Result<(), String>,
    Turn: FnMut(&str, &str) -> Result<ChatTurnResult, String>,
{
    ensure_create_then_turn_with_error(
        prompt,
        "rejected_unbound".to_string(),
        "rejected_unbound".to_string(),
        "rejected_not_live_session".to_string(),
        create,
        turn,
    )
}

/// Typed-error form used by engine adapters while retaining Binding ownership,
/// create-before-turn ordering, and session-id isolation routing.
pub fn ensure_create_then_turn_with_error<E, R, Create, Turn>(
    prompt: &str,
    unbound_error: E,
    invalid_binding_error: E,
    missing_session_error: E,
    mut create: Create,
    mut turn: Turn,
) -> Result<R, E>
where
    E: Clone,
    Create: FnMut(&str) -> Result<(), E>,
    Turn: FnMut(&str, &str) -> Result<R, E>,
{
    let live = session::live_context_owner();
    let binding = live.current_binding().ok_or(unbound_error)?;
    let _business_id = session::binding_business_id(&binding).ok_or(invalid_binding_error)?;
    let session_id = live.current_session_id().ok_or(missing_session_error)?;
    create(&session_id)?;
    turn(&session_id, prompt)
}

/// Cancel only the current live session's Agent turn.
pub fn cancel_from_binding<Cancel>(cancel: Cancel) -> Result<(), String>
where
    Cancel: FnMut(&str) -> Result<(), String>,
{
    cancel_from_binding_with_error(
        "rejected_unbound".to_string(),
        "rejected_unbound".to_string(),
        cancel,
    )
}

/// Typed-error cancellation form. The callback receives the live session id
/// as the Agent isolation key; Binding-derived identity is not exposed.
pub fn cancel_from_binding_with_error<E, Cancel>(
    unbound_error: E,
    invalid_binding_error: E,
    mut cancel: Cancel,
) -> Result<(), E>
where
    E: Clone,
    Cancel: FnMut(&str) -> Result<(), E>,
{
    let binding = current_binding_snapshot().ok_or(unbound_error)?;
    let _business_id =
        session::binding_business_id(&binding).ok_or(invalid_binding_error.clone())?;
    let session_id = session::live_context_owner()
        .current_session_id()
        .ok_or(invalid_binding_error)?;
    // Mark the live chat before invoking the engine-specific cancellation hook.
    // This keeps Host-loop turns cancellable and makes a failed SDK cancel
    // unable to erase the local cancellation intent.
    session::with_live_mut(|live| live.chat_cancelled = true);
    cancel(&session_id)
}

pub fn has_active_binding() -> bool {
    session::live_context_owner().current_binding().is_some()
}

pub fn current_binding_clone() -> Option<session::Binding> {
    session::live_context_owner().current_binding()
}
