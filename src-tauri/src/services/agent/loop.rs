//! Agent Loop + Host open/chat-turn core (single-flight, terminals, history caps).
//! Host business path: LLM tools empty; no process-local tools::dispatch.

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use serde_json::{json, Value};

use crate::services::agent::llm::{self, LlmConfig, LlmError};
use crate::services::agent::session::{self, Session, Turn};
use crate::services::mcp_server_registry::{self, McpServerConfig, McpServerLookupError};
use crate::services::todo_task;

pub use crate::services::agent::session::{
    validate_binding, Binding, BindingStateSummary, SetError,
};

pub const EVENT_TURN_COMPLETED: &str = "ai-assistant:turn-completed";
pub const WINDOW_LABEL: &str = "ai-assistant";
pub const MAX_CLARIFY_ROUNDS: u32 = 5;
pub const MAX_HISTORY_MESSAGES: usize = 20;
pub const MAX_USER_TURNS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    None,
    Business,
    Error,
}

impl Terminal {
    pub fn as_str(self) -> &'static str {
        match self {
            Terminal::None => "none",
            Terminal::Business => "business",
            Terminal::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TurnOutcome {
    pub reply_text: String,
    pub terminal: Terminal,
    pub wrote: bool,
}

#[derive(Debug, Clone)]
pub struct ChatTurnResult {
    pub body: Value,
    pub emit_turn_completed: Option<Value>,
}

/// Binding Contract lifecycle callback payload (E1): event name + optional result category only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleEvent {
    pub event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'static str>,
}

/// Shell sync notice equivalent to `ai-assistant:binding-changed` (Bound/Unbound).
/// Recorded on Host cut/Set paths so core-only calls cannot bypass shell sync observability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShellSyncEvent {
    pub event: &'static str,
    pub state: &'static str,
}

/// Same event name the shell listens for (`commands::ai_assistant::EVENT_BINDING_CHANGED`).
pub const EVENT_SHELL_BINDING_CHANGED: &str = "ai-assistant:binding-changed";

/// Outcome of a gated Binding Contract execute (tools/prompt from current Binding only).
#[derive(Debug, Clone)]
pub struct ExecuteOutcome {
    pub applied_tools: Value,
    pub applied_prompt: Value,
}

/// Stable execute / gate failure categories (L11-AR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecError {
    code: &'static str,
}

impl ExecError {
    pub fn as_code(&self) -> &'static str {
        self.code
    }

    fn rejected_unbound() -> Self {
        Self {
            code: "rejected_unbound",
        }
    }

    fn reset_cancelled() -> Self {
        Self {
            code: "reset_cancelled",
        }
    }

    fn rejected_stale_generation() -> Self {
        Self {
            code: "rejected_stale_generation",
        }
    }
}

type LifecycleListener = Box<dyn Fn(&LifecycleEvent) + Send + 'static>;

/// Orchestration-only Host runtime (single-flight / present / clarify).
/// Binding, MCP capability, live session id, generation, and cancel flags are
/// owned exclusively by [`session::AIAssistantSession`] (L2-A / T-SessionMigrate).
#[derive(Default)]
struct Runtime {
    busy: bool,
    /// True while a Binding Contract execute is in flight.
    executing: bool,
    /// Present fired before main-window listener was ready (L11-AR race heal).
    pending_present: bool,
    clarify_counts: HashMap<String, u32>,
}

fn runtime() -> &'static Mutex<Runtime> {
    static RUNTIME: OnceLock<Mutex<Runtime>> = OnceLock::new();
    RUNTIME.get_or_init(|| Mutex::new(Runtime::default()))
}

fn lifecycle_log() -> &'static Mutex<Vec<LifecycleEvent>> {
    static LOG: OnceLock<Mutex<Vec<LifecycleEvent>>> = OnceLock::new();
    LOG.get_or_init(|| Mutex::new(Vec::new()))
}

fn lifecycle_listener() -> &'static Mutex<Option<LifecycleListener>> {
    static LISTENER: OnceLock<Mutex<Option<LifecycleListener>>> = OnceLock::new();
    LISTENER.get_or_init(|| Mutex::new(None))
}

fn shell_sync_log() -> &'static Mutex<Vec<ShellSyncEvent>> {
    static LOG: OnceLock<Mutex<Vec<ShellSyncEvent>>> = OnceLock::new();
    LOG.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn reset_runtime_for_tests() {
    let mut rt = runtime().lock().unwrap();
    *rt = Runtime::default();
    drop(rt);
    session::reset_live_for_tests();
    lifecycle_log().lock().unwrap().clear();
    *lifecycle_listener().lock().unwrap() = None;
    shell_sync_log().lock().unwrap().clear();
}

pub fn set_busy_for_tests(busy: bool) {
    runtime().lock().unwrap().busy = busy;
}

pub fn is_execute_cancelled_for_tests() -> bool {
    session::live_context_owner().execute_cancelled()
}

pub fn is_chat_cancelled_for_tests() -> bool {
    session::live_context_owner().chat_cancelled()
}

pub fn set_chat_cancelled_for_tests(cancelled: bool) {
    session::with_live_mut(|live| live.chat_cancelled = cancelled);
}

pub fn clear_lifecycle_events_for_tests() {
    lifecycle_log().lock().unwrap().clear();
}

pub fn drain_lifecycle_events() -> Vec<LifecycleEvent> {
    std::mem::take(&mut *lifecycle_log().lock().unwrap())
}

pub fn set_lifecycle_listener_for_tests(listener: Option<LifecycleListener>) {
    *lifecycle_listener().lock().unwrap() = listener;
}

pub fn clear_shell_sync_events_for_tests() {
    shell_sync_log().lock().unwrap().clear();
}

pub fn drain_shell_sync_events() -> Vec<ShellSyncEvent> {
    std::mem::take(&mut *shell_sync_log().lock().unwrap())
}

/// Record shell binding-changed sync for Bound / Unbound transitions on cut/Set paths.
fn emit_shell_binding_changed() {
    let state = if binding_state() == "bound" {
        "bound"
    } else {
        "unbound"
    };
    shell_sync_log().lock().unwrap().push(ShellSyncEvent {
        event: EVENT_SHELL_BINDING_CHANGED,
        state,
    });
}

fn emit_lifecycle(event: &'static str, category: Option<&'static str>) {
    emit_lifecycle_inner(event, category, true);
}

fn emit_lifecycle_inner(event: &'static str, category: Option<&'static str>, invoke_listener: bool) {
    let ev = LifecycleEvent { event, category };
    lifecycle_log().lock().unwrap().push(ev.clone());
    if !invoke_listener {
        return;
    }
    let listener = {
        let mut slot = lifecycle_listener().lock().unwrap();
        slot.take()
    };
    let Some(listener) = listener else {
        return;
    };
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| listener(&ev)));
    *lifecycle_listener().lock().unwrap() = Some(listener);
    if result.is_err() {
        // Record callback_failed without re-entering the panicking listener.
        emit_lifecycle_inner("onError", Some("callback_failed"), false);
    }
}

/// Symmetric in-flight cancel for cut paths: interrupt execute and/or chat when live.
/// Orchestration flags stay on `Runtime`; cancel authority is on `AIAssistantSession`.
fn request_in_flight_cancel(rt: &Runtime, live: &mut session::AIAssistantSession) {
    if rt.executing {
        live.execute_cancelled = true;
    }
    if rt.busy {
        live.chat_cancelled = true;
    }
}

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
    set_binding_with_mcp(binding, None, None)
}

fn set_binding_with_mcp(
    binding: session::Binding,
    loaded_mcp: Option<McpServerConfig>,
    business_id: Option<String>,
) -> Result<(), SetError> {
    if let Err(e) = session::validate_binding(&binding) {
        emit_lifecycle("onError", Some("set_invalid"));
        return Err(e);
    }
    let was_bound = {
        // Lock order: Runtime (orchestration) → AIAssistantSession (live context).
        let rt = runtime().lock().unwrap();
        session::with_live_mut(|live| {
            let was = live.current_binding.is_some();
            // Chat cancel on cut; do not set execute_cancelled here so replace mid-execute
            // still returns rejected_stale_generation (generation advance is the execute interrupt).
            if rt.busy {
                live.chat_cancelled = true;
            }
            live.generation_seq = live.generation_seq.saturating_add(1);
            live.current_generation = Some(live.generation_seq);
            live.current_binding = Some(binding);
            live.current_business_id = business_id;
            live.loaded_mcp_server = loaded_mcp;
            // Session cut: clear live id with generation update (no new-gen + old-session window).
            live.current_session_id = None;
            was
        })
    };
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
    // L1+L2: public key-only Set must not feed business tool handles to the
    // Agent Loop. The business key is already resolved into loaded_mcp_server;
    // Binding.tools stays an empty interface slot (no in-process dispatch).
    let binding = session::Binding {
        tools: json!([]),
        prompt: json!(config.capability_description.clone()),
        callbacks: parsed.callbacks,
    };
    set_binding_with_mcp(binding, Some(config), Some(key))
}

/// The decision-level `McpServerConfig` shape is the shared read form for the
/// Host Agent Loop. Field-level transport schema remains deferred.
pub const SESSION_CAPABILITY_READ_FACE_REGISTRY_SHAPE: bool = true;

/// Must Close Before T4 — A2 confirmed (not narrowed): Host `mcp_server_registry`
/// is the sole lookup source; this face only exposes config already loaded by
/// key-only Set from that table.
pub const SESSION_CAPABILITY_READ_FACE_A2_HOST_REGISTRY_SOLE_LOOKUP: bool = true;

/// Host adapter with empty tools still completes facade open/ensure/chat turns.
pub const HOST_EMPTY_TOOLS_FACADE_USABLE: bool = true;

/// Read-only session capability consumption face for the Host Agent Loop.
/// Returns a detached clone of the MCP Server config loaded by key-only Binding
/// Set. No write path — only Set/Reset lifecycle may change the loaded value.
pub fn session_capability_mcp_config() -> Option<McpServerConfig> {
    session::live_context_owner().loaded_mcp_server()
}

/// Observability alias for the session capability read face.
pub fn loaded_mcp_server() -> Option<McpServerConfig> {
    session_capability_mcp_config()
}

/// Reset: discard current Binding → unbound. Idempotent when already unbound.
/// Clears `current_session_id` (session cut, clear-first); does not wipe disk turns.
/// Unloads session capability MCP config with the Binding.
/// Bound→unbound emits onUnbound; symmetrically cancels in-flight execute and chat.
pub fn reset_binding() -> Result<(), ()> {
    let was_bound = {
        let rt = runtime().lock().unwrap();
        session::with_live_mut(|live| {
            let was = live.current_binding.is_some();
            request_in_flight_cancel(&rt, live);
            live.current_binding = None;
            live.current_business_id = None;
            live.loaded_mcp_server = None;
            live.current_generation = None;
            live.current_session_id = None;
            was
        })
    };
    if was_bound {
        emit_lifecycle("onUnbound", None);
        // Shell sync: Bound→Unbound (defensive_unbound shares this path — no core-only bypass).
        emit_shell_binding_changed();
    }
    Ok(())
}

/// Reset the current Binding and then close its Host-owned Agent slot by the
/// captured live session identity. The identity must be captured before Reset
/// clears the live context.
pub fn reset_binding_with_close<Close>(mut close: Close) -> Result<(), String>
where
    Close: FnMut(&str) -> Result<(), String>,
{
    let session_id = session::live_context_owner().current_session_id();
    reset_binding().map_err(|_| "reset_binding_failed".to_string())?;
    if let Some(session_id) = session_id {
        close(&session_id)?;
    }
    Ok(())
}

/// Confirmed Host hook path for defensive cut (P2 Must Close Before).
/// Call when leave/unmount was observed as missed while still bound.
pub const DEFENSIVE_CUT_HOOK_PATH: &str =
    "src-tauri/src/services/agent/loop.rs::defensive_unbound";

/// Explicit leave→Reset chain that defensive cut backs (does not replace).
pub const DEFENSIVE_CUT_EXPLICIT_RESET_CHAIN: &[&str] = &[
    "frontend/js/plan-task/index.js::dispose",
    "frontend/js/plan-task/todos-lifecycle.js::onTodosPageLeave",
    "frontend/js/plan-task/todos-binding.js::resetTodosBinding",
    "src-tauri/src/services/agent/loop.rs::reset_binding",
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
fn current_binding_generation_snapshot() -> Option<(session::Binding, u64)> {
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

/// Outcome of Host Present (shell open). Not a Binding Contract result.
/// `window_label` is a Host surface id — it does not imply a WebviewWindow exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentOutcome {
    pub surface: &'static str,
    pub window_label: &'static str,
    pub entry_id: &'static str,
}

/// Present: signal shell to open AI C. Does not Set; does not change binding state.
/// Does not create/focus an independent WebviewWindow (shell host owns presentation).
pub fn present_ai_assistant_core() -> Result<PresentOutcome, String> {
    {
        let mut rt = runtime().lock().unwrap();
        rt.pending_present = true;
    }
    Ok(PresentOutcome {
        surface: "Present",
        window_label: WINDOW_LABEL,
        entry_id: "ai-assistant",
    })
}

/// Shell close (关壳). Not Reset — leaves Binding Contract state unchanged; no onUnbound.
pub fn shell_close_core() -> Result<(), String> {
    Ok(())
}

pub fn clarify_count(session: &Session) -> u32 {
    runtime()
        .lock()
        .unwrap()
        .clarify_counts
        .get(&session.session_id)
        .copied()
        .unwrap_or(0)
}

fn is_clarify_text(content: &str) -> bool {
    let t = content.trim();
    t.contains('？') || t.contains('?')
}

fn turn_to_message(turn: &Turn) -> Value {
    let mut m = json!({ "role": turn.role });
    if let Some(c) = &turn.content {
        m["content"] = json!(c);
    } else if turn.tool_calls.is_none() {
        m["content"] = json!("");
    }
    if let Some(id) = &turn.tool_call_id {
        m["tool_call_id"] = json!(id);
    }
    if let Some(name) = &turn.name {
        m["name"] = json!(name);
    }
    if let Some(tc) = &turn.tool_calls {
        m["tool_calls"] = tc.clone();
    }
    m
}

/// Drop oldest complete user rounds until dual hard caps hold.
pub fn truncate_turns(turns: &[Turn]) -> Vec<Turn> {
    let mut kept = turns.to_vec();
    loop {
        let user_count = kept.iter().filter(|t| t.role == "user").count();
        if kept.len() <= MAX_HISTORY_MESSAGES && user_count <= MAX_USER_TURNS {
            break;
        }
        // Find first user index; drop through next user (exclusive) or end.
        let Some(start) = kept.iter().position(|t| t.role == "user") else {
            break;
        };
        let end = kept[start + 1..]
            .iter()
            .position(|t| t.role == "user")
            .map(|i| start + 1 + i)
            .unwrap_or(kept.len());
        if end == start {
            break;
        }
        kept.drain(start..end);
    }
    kept
}

pub fn build_llm_messages_from_turns(turns: &[Turn], system_prompt: &str) -> Vec<Value> {
    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt,
    })];
    for turn in truncate_turns(turns) {
        messages.push(turn_to_message(&turn));
    }
    messages
}

fn prompt_text_from_binding(prompt: &Value) -> String {
    match prompt {
        Value::String(s) => s.clone(),
        other => other
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| other.to_string()),
    }
}

fn current_binding_snapshot() -> Option<session::Binding> {
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

pub fn map_llm_error(err: &LlmError) -> TurnOutcome {
    let reply_text = match err {
        LlmError::MissingConfig => {
            "Host / GLM is not configured. Enter the API key and model in Settings.".into()
        }
        LlmError::Unauthorized => "鉴权失败，请检查 API Key 配置。".into(),
        LlmError::BadRequest(_) => "请求不被上游接受，请稍后重试或检查配置。".into(),
        LlmError::RateLimited => "请求过于频繁，请稍后重试。".into(),
        LlmError::Server(_) | LlmError::Network(_) => {
            "上游服务暂时不可用，请稍后重试。".into()
        }
        LlmError::Timeout => "调用超时，请稍后重试。".into(),
        LlmError::Truncated => "回复被截断，请重试或缩短请求。".into(),
        LlmError::UnsupportedToolCalls => {
            "当前上游不支持工具调用（tool_calls），无法继续。".into()
        }
        LlmError::InvalidResponse(_) => "模型响应格式异常，未执行任何写入。".into(),
    };
    TurnOutcome {
        reply_text,
        terminal: Terminal::Error,
        wrote: false,
    }
}

pub fn persist(session: &Session) {
    let _ = session::save_session(session);
}

/// Busy / live-session gate + mark busy. `Err` carries the early `ChatTurnResult`.
pub fn try_begin_chat_turn(session_id: &str) -> Result<(), ChatTurnResult> {
    let mut rt = runtime().lock().unwrap();
    if rt.busy {
        return Err(ChatTurnResult {
            body: json!({
                "reply_text": "Busy — try again later",
                "terminal": "none",
                "wrote": false,
                "busy": true,
                "session_id": session_id,
            }),
            emit_turn_completed: None,
        });
    }
    let live_id = session::with_live_mut(|live| live.current_session_id.clone());
    if live_id.as_deref() != Some(session_id) {
        return Err(ChatTurnResult {
            body: json!({
                "reply_text": "Session identity mismatch — cut or stale session cannot continue.",
                "terminal": "business",
                "wrote": false,
                "busy": false,
                "session_id": session_id,
                "code": "rejected_not_live_session",
            }),
            emit_turn_completed: None,
        });
    }
    rt.busy = true;
    session::with_live_mut(|live| live.chat_cancelled = false);
    Ok(())
}

pub fn end_chat_turn_busy() {
    runtime().lock().unwrap().busy = false;
}

pub fn has_active_binding() -> bool {
    session::live_context_owner().current_binding().is_some()
}

pub fn current_binding_clone() -> Option<session::Binding> {
    session::live_context_owner().current_binding()
}

pub fn turn_completed_emit(session_id: &str, wrote: bool, terminal: &str) -> Value {
    emit_payload(session_id, wrote, terminal)
}

/// Executable reject after cut/cancel: return notice in the response only.
/// Do not append/persist business turns on the (possibly cut) session.
fn cancelled_turn_outcome(session: &mut Session, turns_checkpoint: usize) -> TurnOutcome {
    if session.turns.len() != turns_checkpoint {
        session.turns.truncate(turns_checkpoint);
        persist(session);
    }
    TurnOutcome {
        reply_text: "In-flight turn cancelled — binding cut.".to_string(),
        terminal: Terminal::Error,
        wrote: false,
    }
}

fn chat_turn_interrupted(generation: u64) -> bool {
    let live = session::live_context_owner();
    live.chat_cancelled() || live.current_generation() != Some(generation)
}

pub fn run_loop(session: &mut Session, user_message: &str, config: &LlmConfig) -> TurnOutcome {
    let turns_checkpoint = session.turns.len();
    // Executable turns require Binding Contract bound — not session.bound_master_task_id.
    let Some((binding, generation)) = current_binding_generation_snapshot() else {
        session.turns.push(Turn {
            role: "user".into(),
            content: Some(user_message.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        // Distinguish never/fully unbound from a corrupted bound-without-generation slot.
        if current_binding_snapshot().is_none() {
            let reply =
                "Unbound — no active Binding Contract; chat cannot run.".to_string();
            session.turns.push(Turn {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            persist(session);
            return TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Business,
                wrote: false,
            };
        }
        return cancelled_turn_outcome(session, turns_checkpoint);
    };

    let system_prompt = prompt_text_from_binding(&binding.prompt);
    if system_prompt.trim().is_empty() {
        session.turns.push(Turn {
            role: "user".into(),
            content: Some(user_message.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        let reply = "Binding prompt is empty; cannot run chat.".to_string();
        session.turns.push(Turn {
            role: "assistant".into(),
            content: Some(reply.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        persist(session);
        return TurnOutcome {
            reply_text: reply,
            terminal: Terminal::Error,
            wrote: false,
        };
    }

    session.turns.push(Turn {
        role: "user".into(),
        content: Some(user_message.to_string()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    });
    persist(session);

    // Host P3: business path tools always empty; may read L2 MCP face (no tool_calls).
    // Do not call process-local tools::dispatch (L09-I #7 / T3 Failure).
    let _ = session_capability_mcp_config();

    if chat_turn_interrupted(generation) {
        return cancelled_turn_outcome(session, turns_checkpoint);
    }
    let messages = build_llm_messages_from_turns(&session.turns, &system_prompt);
    let msg = match llm::chat_completions(&messages, &[], config) {
        Ok(m) => m,
        Err(e) => {
            if chat_turn_interrupted(generation) {
                return cancelled_turn_outcome(session, turns_checkpoint);
            }
            let out = map_llm_error(&e);
            session.turns.push(Turn {
                role: "assistant".into(),
                content: Some(out.reply_text.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            persist(session);
            return out;
        }
    };

    // Round-trip gate: cancel / generation may have been raised during LLM.
    if chat_turn_interrupted(generation) {
        return cancelled_turn_outcome(session, turns_checkpoint);
    }

    if !msg.tool_calls.is_empty() {
        // Unexpected tool_calls with empty request tools — never dispatch in-process.
        let reply = "模型响应异常或请求了不支持的工具，未执行任何写入。".to_string();
        session.turns.push(Turn {
            role: "assistant".into(),
            content: Some(reply.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        persist(session);
        return TurnOutcome {
            reply_text: reply,
            terminal: Terminal::Error,
            wrote: false,
        };
    }

    let content = msg.content.clone().unwrap_or_default();
    if content.trim().is_empty() {
        let reply = "模型响应为空，未执行任何写入。".to_string();
        session.turns.push(Turn {
            role: "assistant".into(),
            content: Some(reply.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        persist(session);
        return TurnOutcome {
            reply_text: reply,
            terminal: Terminal::Error,
            wrote: false,
        };
    }

    if is_clarify_text(&content) {
        let mut rt = runtime().lock().unwrap();
        let count = rt
            .clarify_counts
            .entry(session.session_id.clone())
            .or_insert(0);
        if *count >= MAX_CLARIFY_ROUNDS {
            drop(rt);
            let reply = "澄清次数已达上限，请换种方式说明需求或稍后重试。".to_string();
            session.turns.push(Turn {
                role: "assistant".into(),
                content: Some(reply.clone()),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            persist(session);
            return TurnOutcome {
                reply_text: reply,
                terminal: Terminal::Error,
                wrote: false,
            };
        }
        *count += 1;
        drop(rt);
        session.turns.push(Turn {
            role: "assistant".into(),
            content: Some(content.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        persist(session);
        return TurnOutcome {
            reply_text: content,
            terminal: Terminal::None,
            wrote: false,
        };
    }

    if content.contains("目前不支持") {
        session.turns.push(Turn {
            role: "assistant".into(),
            content: Some(content.clone()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        persist(session);
        return TurnOutcome {
            reply_text: content,
            terminal: Terminal::Business,
            wrote: false,
        };
    }

    session.turns.push(Turn {
        role: "assistant".into(),
        content: Some(content.clone()),
        tool_call_id: None,
        tool_calls: None,
        name: None,
    });
    persist(session);
    TurnOutcome {
        reply_text: content,
        terminal: Terminal::None,
        wrote: false,
    }
}

fn plan_title(master_task_id: &str) -> Option<String> {
    let got = todo_task::get_by_id(master_task_id);
    if got
        .get("_status")
        .and_then(|s| s.as_u64())
        .is_some_and(|s| s >= 400)
    {
        return None;
    }
    got.get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Current chat session for the assistant window (may be empty if never opened).
/// Does not expose business master id / title (stripped from Host surface).
/// `pending_present` is taken (cleared) on pull so mount can heal Present-before-listen races.
/// Extends with live-session `turns` (read-only hydrate via `session::load_session`);
/// empty when no live / after Reset.
pub fn get_ai_assistant_binding_core() -> Value {
    let (session_id, busy, pending_present) = {
        let mut rt = runtime().lock().unwrap();
        let pending_present = std::mem::take(&mut rt.pending_present);
        let session_id = session::with_live_mut(|live| {
            live.current_session_id.clone().unwrap_or_default()
        });
        (session_id, rt.busy, pending_present)
    };
    let turns = session::load_turns_value(&session_id);
    json!({
        "session_id": session_id,
        "window_label": WINDOW_LABEL,
        "busy": busy,
        "pending_present": pending_present,
        "turns": turns,
    })
}

/// Ensure a chat session exists for turn history only (no master / title).
pub fn ensure_chat_session_core() -> Result<Value, String> {
    {
        let rt = runtime().lock().unwrap();
        let existing = session::with_live_mut(|live| live.current_session_id.clone());
        if let Some(ref sid) = existing {
            if session::load_session(sid).is_ok() {
                return Ok(json!({
                    "session_id": sid,
                    "window_label": WINDOW_LABEL,
                    "busy": rt.busy,
                }));
            }
        }
    }
    let sess = session::create_session(None, None)?;
    let _rt = runtime().lock().unwrap();
    session::with_live_mut(|live| {
        live.current_session_id = Some(sess.session_id.clone());
        live.bound_master_task_id = None;
        live.bound_title = None;
    });
    Ok(json!({
        "session_id": sess.session_id,
        "window_label": WINDOW_LABEL,
        "busy": false,
    }))
}

/// Legacy open path: create/focus session without writing business master into Host runtime.
/// Prefer Present + Binding Contract Set; this no longer binds a plan id.
pub fn open_ai_assistant_core(master_task_id: &str) -> Result<Value, String> {
    let id = master_task_id.trim();
    if id.is_empty() {
        return Err("Missing master_task_id".into());
    }
    // Validate plan exists for legacy callers, but do not store id on Host/session.
    let _title = plan_title(id).ok_or_else(|| "Plan not found".to_string())?;

    let mut rt = runtime().lock().unwrap();
    if rt.busy {
        let sid = session::with_live_mut(|live| live.current_session_id.clone());
        return Ok(json!({
            "session_id": sid,
            "window_label": WINDOW_LABEL,
            "busy": true,
            "reply_text": "Busy — try again later",
        }));
    }

    let sess = session::create_session(None, None)?;
    session::with_live_mut(|live| {
        live.current_session_id = Some(sess.session_id.clone());
        live.bound_master_task_id = None;
        live.bound_title = None;
    });
    rt.clarify_counts.insert(sess.session_id.clone(), 0);

    Ok(json!({
        "session_id": sess.session_id,
        "window_label": WINDOW_LABEL,
        "busy": false,
    }))
}

fn emit_payload(
    session_id: &str,
    wrote: bool,
    terminal: &str,
) -> Value {
    json!({
        "event": EVENT_TURN_COMPLETED,
        "payload": {
            "session_id": session_id,
            "wrote": wrote,
            "terminal": terminal,
        }
    })
}

/// Compatibility entry — production formal chat goes through `runtime::chat_turn`.
/// Delegates so Host loop tests keep a stable symbol while orchestration is engine-aware.
pub fn agent_chat_turn_core(
    session_id: &str,
    message: &str,
    master_task_id: Option<&str>,
) -> Result<ChatTurnResult, String> {
    crate::services::agent::runtime::chat_turn(session_id, message, master_task_id)
}
