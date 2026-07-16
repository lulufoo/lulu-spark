//! Host commands: open_ai_assistant / agent_chat_turn.

use serde_json::Value;
use tauri::{AppHandle, Emitter};

use crate::services::agent::r#loop::{self, ChatTurnResult, EVENT_TURN_COMPLETED, WINDOW_LABEL};

pub const AI_ASSISTANT_WINDOW_LABEL: &str = WINDOW_LABEL;

pub fn open_ai_assistant_json(master_task_id: &str) -> Result<Value, String> {
    r#loop::open_ai_assistant_core(master_task_id)
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
    _app: AppHandle,
    master_task_id: String,
) -> Result<Value, String> {
    // Window create-or-focus lands in t5; binding/session is owned here.
    tauri::async_runtime::spawn_blocking(move || open_ai_assistant_json(&master_task_id))
        .await
        .map_err(|e| e.to_string())?
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
