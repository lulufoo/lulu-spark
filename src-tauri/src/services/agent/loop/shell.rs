//! Present / close / get_binding / open / list / select / create / chat-turn entry.

use serde_json::{json, Value};

use crate::services::agent::session;
use crate::services::todo_task;

use super::flights::runtime;
use super::types::{ChatTurnResult, PresentOutcome, WINDOW_LABEL};

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

fn plan_title(master_task_id: &str) -> Option<String> {
    let got = match todo_task::get_by_id(master_task_id) {
        Ok(value) => value,
        Err(_) => return None,
    };
    got.get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Current chat session for the assistant window (may be empty if never opened).
/// Does not expose business master id / title (stripped from Host surface).
/// `pending_present` is taken (cleared) on pull so mount can heal Present-before-listen races.
/// Extends with live-session display `turns` (user/assistant text via
/// `session::load_turns_value`); empty when no live / after Reset.
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
    let staged = session::load_staged_value(&session_id);
    json!({
        "session_id": session_id,
        "window_label": WINDOW_LABEL,
        "busy": busy,
        "pending_present": pending_present,
        "turns": turns,
        "staged": staged,
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
    let sess = session::create_session()?;
    let _rt = runtime().lock().unwrap();
    session::with_live_mut(|live| {
        live.current_session_id = Some(sess.session_id.clone());
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

    let sess = session::create_session()?;
    session::with_live_mut(|live| {
        live.current_session_id = Some(sess.session_id.clone());
    });
    rt.clarify_counts.insert(sess.session_id.clone(), 0);

    Ok(json!({
        "session_id": sess.session_id,
        "window_label": WINDOW_LABEL,
        "busy": false,
    }))
}

/// Home history list. Does not change the live session.
pub fn list_chat_sessions_core() -> Result<Value, String> {
    let sessions = session::list_session_summaries()?;
    let current_session_id = session::with_live_mut(|live| live.current_session_id.clone());
    Ok(json!({
        "sessions": sessions,
        "current_session_id": current_session_id,
    }))
}

/// Make an existing disk session the live chat session. Does not Set Binding.
pub fn select_chat_session_core(session_id: &str) -> Result<Value, String> {
    let id = session_id.trim();
    if id.is_empty() {
        return Err("Missing session_id".into());
    }
    let _ = session::load_session(id)?;
    session::with_live_mut(|live| {
        live.current_session_id = Some(id.to_string());
    });
    Ok(get_ai_assistant_binding_core())
}

/// Start a blank chat session and make it live. Does not Set Binding.
pub fn create_chat_session_core() -> Result<Value, String> {
    let sess = session::create_session()?;
    session::with_live_mut(|live| {
        live.current_session_id = Some(sess.session_id.clone());
    });
    Ok(get_ai_assistant_binding_core())
}

/// Remove a disk session. Rejects an in-flight turn. Clears live id when it matches.
pub fn delete_chat_session_core(session_id: &str) -> Result<Value, String> {
    let id = session_id.trim();
    if id.is_empty() {
        return Err("Missing session_id".into());
    }
    {
        let rt = runtime().lock().unwrap();
        if rt.flights.contains_key(id) {
            return Err("Conversation is running".into());
        }
    }
    session::delete_session(id)?;
    session::with_live_mut(|live| {
        if live.current_session_id.as_deref() == Some(id) {
            live.current_session_id = None;
        }
    });
    list_chat_sessions_core()
}

/// UI-only: remove one Stage entry by F-handle. Rejects in-flight turns. Not an agent/MCP tool.
pub fn unstage_chat_staged_core(session_id: &str, staged_id: &str) -> Result<Value, String> {
    let sid = session_id.trim();
    if sid.is_empty() {
        return Err("Missing session_id".into());
    }
    {
        let rt = runtime().lock().unwrap();
        if rt.flights.contains_key(sid) {
            return Err("Conversation is running".into());
        }
    }
    let session = session::unstage_entry(sid, staged_id)?;
    Ok(json!({
        "session_id": session.session_id,
        "staged": session.staged,
    }))
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
