//! Host commands: open_ai_assistant / Present / agent_chat_turn.
//! Dual surface: Binding Contract ops (Set/Reset/query/execute + callbacks) vs shell Present.

use std::time::Instant;

use serde_json::{json, Value};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter};

use crate::services::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::services::agent::progress::{ProgressDesc, ProgressSink};
use crate::services::agent::r#loop::{self, ChatTurnResult, EVENT_TURN_COMPLETED, WINDOW_LABEL};
use crate::services::agent::runtime;
pub use crate::services::agent::session::value_exposes_engine_selection;

pub const AI_ASSISTANT_WINDOW_LABEL: &str = WINDOW_LABEL;
pub const EVENT_ASSISTANT_OPENED: &str = "ai-assistant:opened";
/// Emitted when Binding Contract Set/Reset/defensive cut changes `query_binding` state (shell composer gate).
pub const EVENT_BINDING_CHANGED: &str = "ai-assistant:binding-changed";

/// Engine-opaque session facade — public open/ensure/chat entry points never
/// accept channel-selection parameters; callers cannot branch on the assistant
/// channel via signatures or return/event payload fields.
pub const SESSION_FACADE_ENGINE_OPAQUE: bool = true;

pub fn open_ai_assistant_json(master_task_id: &str) -> Result<Value, String> {
    // Physical open path may still carry shell UX payload; Present semantics must not imply Set.
    r#loop::open_ai_assistant_core(master_task_id)
}

/// Provision chat session for turn history only (no master / title).
/// Used after Binding Contract Set so shell can `agent_chat_turn` while Present≠Set.
pub fn ensure_ai_assistant_session_json() -> Result<Value, String> {
    r#loop::ensure_chat_session_core()
}

/// Host Present (shell open signal). Not a Binding Contract op; does not Set or change binding state.
pub fn present_ai_assistant_json() -> Result<Value, String> {
    let outcome = r#loop::present_ai_assistant_core()?;
    Ok(json!({
        "surface": outcome.surface,
        "window_label": outcome.window_label,
        "entry_id": outcome.entry_id,
    }))
}

/// Shell close notification. 关壳 ≠ Reset — does not unbound / does not emit onUnbound.
pub fn shell_close_json() -> Value {
    let _ = r#loop::shell_close_core();
    json!({
        "ok": true,
        "state": r#loop::binding_state(),
    })
}

pub fn get_ai_assistant_binding_json() -> Value {
    r#loop::get_ai_assistant_binding_core()
}

pub fn list_chat_sessions_json() -> Result<Value, String> {
    let value = r#loop::list_chat_sessions_core()?;
    let count = value
        .get("sessions")
        .and_then(|s| s.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    eprintln!("[DEBUG-assistant] host: list_chat_sessions count={count}");
    Ok(value)
}

pub fn select_chat_session_json(session_id: &str) -> Result<Value, String> {
    r#loop::select_chat_session_core(session_id)
}

pub fn create_chat_session_json() -> Result<Value, String> {
    r#loop::create_chat_session_core()
}

/// Binding Contract Set entry (key-only). Looks up Host MCP registry; rejects legacy
/// tools/prompt/callbacks payload and engine selection parameters.
pub fn set_binding_json(binding: Value) -> Value {
    eprintln!(
        "[DEBUG-assistant] host: set_binding_json in key={:?}",
        binding.get("key")
    );
    match r#loop::try_set_binding_json(&binding) {
        Ok(()) => {
            let state = r#loop::binding_state();
            eprintln!("[DEBUG-assistant] host: set_binding_json ok state={state}");
            json!({
                "ok": true,
                "state": state,
            })
        }
        Err(e) => {
            let state = r#loop::binding_state();
            eprintln!(
                "[DEBUG-assistant] host: set_binding_json err code={} state={state}",
                e.as_code()
            );
            json!({
                "ok": false,
                "code": e.as_code(),
                "state": state,
            })
        }
    }
}

fn unbound_ok_json() -> Value {
    json!({
        "ok": true,
        "state": r#loop::binding_state(),
    })
}

/// Binding Contract Reset → unbound (idempotent).
pub fn reset_binding_json() -> Value {
    let _ = runtime::reset_binding();
    unbound_ok_json()
}

/// Host defensive cut → unbound (same semantics as Reset). Emits shell sync via core + command emit.
pub fn defensive_unbound_json() -> Value {
    let _ = runtime::reset_binding();
    unbound_ok_json()
}

/// Binding Contract read-only query (business-agnostic summary).
pub fn query_binding_json() -> Value {
    let value = serde_json::to_value(r#loop::query_binding())
        .unwrap_or_else(|_| json!({ "state": "unbound" }));
    eprintln!("[DEBUG-assistant] host: query_binding {value}");
    value
}

/// Binding Contract execute gate: requires bound; applies current Binding tools/prompt.
pub fn execute_binding_json() -> Value {
    match r#loop::execute_binding() {
        Ok(out) => json!({
            "ok": true,
            "applied_tools": out.applied_tools,
            "applied_prompt": out.applied_prompt,
            "state": r#loop::binding_state(),
        }),
        Err(e) => json!({
            "ok": false,
            "code": e.as_code(),
            "state": r#loop::binding_state(),
        }),
    }
}

/// Cancel the current Binding's in-flight chat turn without closing its Agent slot.
pub fn cancel_ai_assistant_turn_json() -> Value {
    match runtime::cancel_from_binding() {
        Ok(()) => json!({
            "ok": true,
            "cancelled": true,
            "state": r#loop::binding_state(),
        }),
        Err(error) => json!({
            "ok": false,
            "cancelled": false,
            "error": error,
            "state": r#loop::binding_state(),
        }),
    }
}

/// Invokable Binding Contract Set (key-only; no engine selection parameter).
#[tauri::command]
pub async fn set_binding(app: AppHandle, binding: Value) -> Result<Value, String> {
    eprintln!("[DEBUG-assistant] host: set_binding command");
    let result = tauri::async_runtime::spawn_blocking(move || set_binding_json(binding))
        .await
        .map_err(|e| e.to_string())?;
    eprintln!(
        "[DEBUG-assistant] host: emit {} ok={:?}",
        EVENT_BINDING_CHANGED,
        result.get("ok")
    );
    let _ = app.emit(EVENT_BINDING_CHANGED, &result);
    Ok(result)
}

/// Invokable Binding Contract Reset → unbound (no engine selection parameter).
#[tauri::command]
pub async fn reset_binding(app: AppHandle) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(reset_binding_json)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(EVENT_BINDING_CHANGED, &result);
    Ok(result)
}

/// Invokable Host defensive cut → unbound (shell sync emit; must not be core-only).
#[tauri::command]
pub async fn defensive_unbound(app: AppHandle) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(defensive_unbound_json)
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(EVENT_BINDING_CHANGED, &result);
    Ok(result)
}

/// Invokable Binding Contract query (business-agnostic).
#[tauri::command]
pub async fn query_binding() -> Result<Value, String> {
    Ok(tauri::async_runtime::spawn_blocking(query_binding_json)
        .await
        .map_err(|e| e.to_string())?)
}

/// Invokable Binding Contract execute gate.
#[tauri::command]
pub async fn execute_binding() -> Result<Value, String> {
    Ok(tauri::async_runtime::spawn_blocking(execute_binding_json)
        .await
        .map_err(|e| e.to_string())?)
}

/// Invokable chat cancellation; does not reset Binding or dispose the Agent.
#[tauri::command]
pub async fn cancel_ai_assistant_turn() -> Result<Value, String> {
    Ok(
        tauri::async_runtime::spawn_blocking(cancel_ai_assistant_turn_json)
            .await
            .map_err(|e| e.to_string())?,
    )
}

pub fn agent_chat_turn_json(
    session_id: &str,
    message: &str,
    master_task_id: Option<&str>,
) -> Result<ChatTurnResult, String> {
    // Channel-aware orchestration stays behind the engine-opaque public facade.
    runtime::chat_turn(session_id, message, master_task_id)
}

fn agent_chat_turn_with_trace(
    session_id: &str,
    message: &str,
    master_task_id: Option<&str>,
    trace_id: &TraceId,
    sink: ProgressSink,
) -> Result<ChatTurnResult, String> {
    runtime::chat_turn_with_trace(session_id, message, master_task_id, trace_id, sink)
}

#[tauri::command]
pub async fn open_ai_assistant(
    app: AppHandle,
    master_task_id: String,
) -> Result<Value, String> {
    // Legacy open+bind path (writes bound_master_task_id via open_ai_assistant_core).
    // Todos page Present must use present_ai_assistant instead (L2 t3 / L06-T).
    let result = tauri::async_runtime::spawn_blocking(move || open_ai_assistant_json(&master_task_id))
        .await
        .map_err(|e| e.to_string())??;

    #[cfg(not(test))]
    {
        // Independent window retired (T4); emit for main-shell listeners only.
        // First-open races are healed by get_ai_assistant_binding on mount.
        let _ = app.emit(EVENT_ASSISTANT_OPENED, &result);
    }
    #[cfg(test)]
    {
        let _ = &app;
    }

    Ok(result)
}

/// Present: emit shell open signal for AI C (`entry_id`). Does not create/focus a WebviewWindow.
/// Not a Binding Contract op — does not Set, does not write bound_master_task_id, does not change binding state.
/// Todos page Assistant button must call this (not `open_ai_assistant`).
#[tauri::command]
pub async fn present_ai_assistant(app: AppHandle) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(present_ai_assistant_json)
        .await
        .map_err(|e| e.to_string())??;

    #[cfg(not(test))]
    {
        // Present is main-window listen → navigate Home; no independent window.
        let _ = app.emit(EVENT_ASSISTANT_OPENED, &result);
    }
    #[cfg(test)]
    {
        let _ = &app;
    }

    Ok(result)
}

/// Ensure a chat session exists without Present / window focus / master write.
/// Does not Set Binding Contract. Used after consumer Set so shell can turn.
#[tauri::command]
pub async fn ensure_ai_assistant_session(app: AppHandle) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(ensure_ai_assistant_session_json)
        .await
        .map_err(|e| e.to_string())??;

    #[cfg(not(test))]
    {
        // Notify an already-open shell; do not create/focus the window (Present≠ensure).
        let _ = app.emit(EVENT_ASSISTANT_OPENED, &result);
    }
    #[cfg(test)]
    {
        let _ = &app;
    }

    Ok(result)
}

/// Notify Host that the assistant shell closed. 关壳 ≠ Reset.
#[tauri::command]
pub async fn shell_close_ai_assistant() -> Result<Value, String> {
    Ok(tauri::async_runtime::spawn_blocking(shell_close_json)
        .await
        .map_err(|e| e.to_string())?)
}

/// Pull current binding after the assistant window mounts (heals emit race on first open).
#[tauri::command]
pub async fn get_ai_assistant_binding() -> Result<Value, String> {
    Ok(tauri::async_runtime::spawn_blocking(get_ai_assistant_binding_json)
        .await
        .map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn list_chat_sessions() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(list_chat_sessions_json)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn select_chat_session(session_id: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || select_chat_session_json(&session_id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn create_chat_session() -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(create_chat_session_json)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn agent_chat_turn(
    app: AppHandle,
    session_id: String,
    message: String,
    master_task_id: Option<String>,
    trace_id: Option<String>,
    progress: Channel<ProgressDesc>,
) -> Result<Value, String> {
    let trace_id = TraceId::from_optional(trace_id);
    let command_started = Instant::now();
    let worker_trace_id = trace_id.clone();
    let sink: ProgressSink = std::sync::Arc::new(move |desc: ProgressDesc| {
        let _ = progress.send(desc);
    });
    let session_id_for_log = session_id.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let worker_started = Instant::now();
        let _ = diagnostics::log(
            DiagnosticEvent::point(
                "assistant.command",
                "blocking_worker.started",
                &worker_trace_id,
            )
            .with_session_id(&session_id),
        );
        let result = agent_chat_turn_with_trace(
            &session_id,
            &message,
            master_task_id.as_deref(),
            &worker_trace_id,
            sink,
        );
        let outcome = if result.is_ok() { "ok" } else { "error" };
        let _ = diagnostics::log(
            DiagnosticEvent::timing(
                "assistant.command",
                "blocking_worker.completed",
                &worker_trace_id,
                worker_started.elapsed(),
            )
            .with_session_id(&session_id)
            .with_static_field("outcome", outcome),
        );
        result
    })
    .await;

    let result = match result {
        Ok(Ok(result)) => {
            let _ = diagnostics::log(
                DiagnosticEvent::timing(
                    "assistant.command",
                    "agent_chat_turn.completed",
                    &trace_id,
                    command_started.elapsed(),
                )
                .with_session_id(&session_id_for_log)
                .with_static_field("outcome", "ok"),
            );
            result
        }
        Ok(Err(err)) => {
            let _ = diagnostics::log(
                DiagnosticEvent::timing(
                    "assistant.command",
                    "agent_chat_turn.completed",
                    &trace_id,
                    command_started.elapsed(),
                )
                .with_session_id(&session_id_for_log)
                .with_static_field("outcome", "error"),
            );
            return Err(err);
        }
        Err(err) => {
            let _ = diagnostics::log(
                DiagnosticEvent::timing(
                    "assistant.command",
                    "agent_chat_turn.completed",
                    &trace_id,
                    command_started.elapsed(),
                )
                .with_session_id(&session_id_for_log)
                .with_static_field("outcome", "task_error"),
            );
            return Err(err.to_string());
        }
    };

    if let Some(ev) = &result.emit_turn_completed {
        if let Some(payload) = ev.get("payload") {
            let _ = app.emit(EVENT_TURN_COMPLETED, payload);
        }
    }
    Ok(result.body)
}

#[cfg(test)]
#[path = "../unit-tests/commands/ai_assistant.rs"]
mod tests;
