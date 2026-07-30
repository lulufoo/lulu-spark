//! Host commands: open_ai_assistant / Present / agent_chat_turn.
//! Dual surface: Binding Contract ops (Set/Reset/query/execute + callbacks) vs shell Present.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::services::agent::r#loop::{self, ChatTurnResult, EVENT_TURN_COMPLETED, WINDOW_LABEL};

pub const AI_ASSISTANT_WINDOW_LABEL: &str = WINDOW_LABEL;
pub const EVENT_ASSISTANT_OPENED: &str = "ai-assistant:opened";
/// Emitted when Binding Contract Set/Reset changes `query_binding` state (shell composer gate).
pub const EVENT_BINDING_CHANGED: &str = "ai-assistant:binding-changed";

pub fn open_ai_assistant_json(master_task_id: &str) -> Result<Value, String> {
    // Physical open path may still carry shell UX payload; Present semantics must not imply Set.
    r#loop::open_ai_assistant_core(master_task_id)
}

/// Provision chat session for a master without Present/open window.
/// Used by Todos after Binding Contract Set so shell can `agent_chat_turn` while Present≠Set.
pub fn ensure_ai_assistant_session_json(master_task_id: &str) -> Result<Value, String> {
    r#loop::open_ai_assistant_core(master_task_id)
}

/// Host Present (shell open/focus). Not a Binding Contract op; does not Set or change binding state.
pub fn present_ai_assistant_json() -> Result<Value, String> {
    let outcome = r#loop::present_ai_assistant_core()?;
    Ok(json!({
        "surface": outcome.surface,
        "window_label": outcome.window_label,
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

/// Binding Contract Set entry (tools + prompt + callbacks). Does not fill tools/prompt.
pub fn set_binding_json(binding: Value) -> Value {
    match r#loop::try_set_binding_json(&binding) {
        Ok(()) => json!({
            "ok": true,
            "state": r#loop::binding_state(),
        }),
        Err(e) => json!({
            "ok": false,
            "code": e.as_code(),
            "state": r#loop::binding_state(),
        }),
    }
}

/// Binding Contract Reset → unbound (idempotent).
pub fn reset_binding_json() -> Value {
    let _ = r#loop::reset_binding();
    json!({
        "ok": true,
        "state": r#loop::binding_state(),
    })
}

/// Binding Contract read-only query (business-agnostic summary).
pub fn query_binding_json() -> Value {
    serde_json::to_value(r#loop::query_binding()).unwrap_or_else(|_| json!({ "state": "unbound" }))
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

/// Invokable Binding Contract Set. Host does not assemble tools/prompt.
#[tauri::command]
pub async fn set_binding(app: AppHandle, binding: Value) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(move || set_binding_json(binding))
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit(EVENT_BINDING_CHANGED, &result);
    Ok(result)
}

/// Invokable Binding Contract Reset → unbound.
#[tauri::command]
pub async fn reset_binding(app: AppHandle) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(reset_binding_json)
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

pub fn agent_chat_turn_json(
    session_id: &str,
    message: &str,
    master_task_id: Option<&str>,
) -> Result<ChatTurnResult, String> {
    r#loop::agent_chat_turn_core(session_id, message, master_task_id)
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
        crate::create_or_focus_ai_assistant_window(&app).map_err(|e| e.to_string())?;
        // Existing windows with a listener already attached receive this.
        // First-open races are healed by get_ai_assistant_binding on mount.
        let _ = app.emit(EVENT_ASSISTANT_OPENED, &result);
    }
    #[cfg(test)]
    {
        let _ = &app;
    }

    Ok(result)
}

/// Present: open or focus the assistant shell. Maps to `create_or_focus_ai_assistant_window`.
/// Not a Binding Contract op — does not Set, does not write bound_master_task_id, does not change binding state.
/// Todos page Assistant button must call this (not `open_ai_assistant`).
#[tauri::command]
pub async fn present_ai_assistant(app: AppHandle) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(present_ai_assistant_json)
        .await
        .map_err(|e| e.to_string())??;

    #[cfg(not(test))]
    {
        crate::create_or_focus_ai_assistant_window(&app).map_err(|e| e.to_string())?;
    }
    #[cfg(test)]
    {
        let _ = &app;
    }

    Ok(result)
}

/// Ensure a chat session exists for `master_task_id` without Present / window focus.
/// Does not Set Binding Contract. Used after consumer Set so shell composer can turn.
#[tauri::command]
pub async fn ensure_ai_assistant_session(
    app: AppHandle,
    master_task_id: String,
) -> Result<Value, String> {
    let result =
        tauri::async_runtime::spawn_blocking(move || ensure_ai_assistant_session_json(&master_task_id))
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
pub async fn agent_chat_turn(
    app: AppHandle,
    session_id: String,
    message: String,
    master_task_id: Option<String>,
) -> Result<Value, String> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        agent_chat_turn_json(
            &session_id,
            &message,
            master_task_id.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())??;

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
