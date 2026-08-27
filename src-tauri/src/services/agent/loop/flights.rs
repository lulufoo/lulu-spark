//! Flight table, busy gate, and Binding lifecycle emit.

use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::json;

use crate::services::agent::diagnostics::TraceId;
use crate::services::agent::progress::ProgressSink;
use crate::services::agent::session::{self, Session};

use super::types::{
    ChatTurnResult, LifecycleEvent, ShellSyncEvent, EVENT_SHELL_BINDING_CHANGED, MAX_CHAT_FLIGHTS,
};

type LifecycleListener = Box<dyn Fn(&LifecycleEvent) + Send + 'static>;

#[allow(dead_code)]
pub(super) struct Flight {
    #[allow(dead_code)]
    pub(super) request_id: String,
    /// Same sink the loop holds on the stack; kept so a row is a full Flight.
    #[allow(dead_code)]
    pub(super) sink: ProgressSink,
    pub(super) cancel: Arc<AtomicBool>,
}

/// Orchestration-only Host runtime (flight table / present / clarify).
/// Binding, MCP capability, live session id, generation, and cancel flags are
/// owned exclusively by [`session::AIAssistantSession`] (L2-A / T-SessionMigrate).
#[derive(Default)]
pub(super) struct Runtime {
    pub(super) busy: bool,
    /// True while a Binding Contract execute is in flight.
    pub(super) executing: bool,
    /// Present fired before main-window listener was ready (L11-AR race heal).
    pub(super) pending_present: bool,
    pub(super) clarify_counts: HashMap<String, u32>,
    pub(super) flights: HashMap<String, Flight>,
}

pub(super) fn runtime() -> &'static Mutex<Runtime> {
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
pub(super) fn emit_shell_binding_changed() {
    let state = if session::live_context_owner().current_binding().is_some() {
        "bound"
    } else {
        "unbound"
    };
    shell_sync_log().lock().unwrap().push(ShellSyncEvent {
        event: EVENT_SHELL_BINDING_CHANGED,
        state,
    });
}

pub(super) fn emit_lifecycle(event: &'static str, category: Option<&'static str>) {
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
pub(super) fn request_in_flight_cancel(rt: &Runtime, live: &mut session::AIAssistantSession) {
    if rt.executing {
        live.execute_cancelled = true;
    }
    if rt.busy || !rt.flights.is_empty() {
        live.chat_cancelled = true;
        for flight in rt.flights.values() {
            flight.cancel.store(true, Ordering::Relaxed);
        }
    }
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

fn busy_turn_result(session_id: &str) -> ChatTurnResult {
    ChatTurnResult {
        body: json!({
            "reply_text": "Busy — try again later",
            "terminal": "none",
            "wrote": false,
            "busy": true,
            "session_id": session_id,
        }),
        emit_turn_completed: None,
    }
}

/// Flight gate + live-session check. `Err` carries the early `ChatTurnResult`.
pub fn try_begin_chat_turn(
    session_id: &str,
    request_id: &TraceId,
    sink: ProgressSink,
) -> Result<(), ChatTurnResult> {
    let mut rt = runtime().lock().unwrap();
    if rt.flights.contains_key(session_id) {
        return Err(busy_turn_result(session_id));
    }
    if rt.flights.len() >= MAX_CHAT_FLIGHTS {
        return Err(busy_turn_result(session_id));
    }
    // Test hook: `set_busy_for_tests(true)` with an empty table still rejects.
    if rt.busy && rt.flights.is_empty() {
        return Err(busy_turn_result(session_id));
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
    rt.flights.insert(
        session_id.to_string(),
        Flight {
            request_id: request_id.as_str().to_string(),
            sink: Arc::clone(&sink),
            cancel: Arc::new(AtomicBool::new(false)),
        },
    );
    rt.busy = true;
    if rt.flights.len() == 1 {
        session::with_live_mut(|live| live.chat_cancelled = false);
    }
    Ok(())
}

pub fn end_chat_turn(session_id: &str) {
    let mut rt = runtime().lock().unwrap();
    rt.flights.remove(session_id);
    rt.busy = !rt.flights.is_empty();
}

pub fn end_chat_turn_busy() {
    let mut rt = runtime().lock().unwrap();
    rt.flights.clear();
    rt.busy = false;
}

pub(super) fn chat_turn_interrupted(session_id: &str, generation: u64) -> bool {
    let live = session::live_context_owner();
    if live.chat_cancelled() || live.current_generation() != Some(generation) {
        return true;
    }
    runtime()
        .lock()
        .unwrap()
        .flights
        .get(session_id)
        .map(|flight| flight.cancel.load(Ordering::Relaxed))
        .unwrap_or(false)
}
