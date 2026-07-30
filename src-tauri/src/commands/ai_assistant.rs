//! Host commands: open_ai_assistant / agent_chat_turn.

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::services::agent::r#loop::{self, ChatTurnResult, EVENT_TURN_COMPLETED, WINDOW_LABEL};

pub const AI_ASSISTANT_WINDOW_LABEL: &str = WINDOW_LABEL;
pub const EVENT_ASSISTANT_OPENED: &str = "ai-assistant:opened";

pub fn open_ai_assistant_json(master_task_id: &str) -> Result<Value, String> {
    r#loop::open_ai_assistant_core(master_task_id)
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
