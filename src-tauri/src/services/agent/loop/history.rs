//! History caps, LLM message build, persist, and cancelled-turn outcome.

use serde_json::{json, Value};

use crate::services::agent::llm::LlmError;
use crate::services::agent::session::{self, Session, Turn};

use super::types::{Terminal, TurnOutcome, EVENT_TURN_COMPLETED, MAX_HISTORY_MESSAGES, MAX_USER_TURNS};

pub(super) fn is_clarify_text(content: &str) -> bool {
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

pub(super) fn prompt_text_from_binding(prompt: &Value) -> String {
    match prompt {
        Value::String(s) => s.clone(),
        other => other
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| other.to_string()),
    }
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

pub fn turn_completed_emit(session_id: &str, wrote: bool, terminal: &str) -> Value {
    emit_payload(session_id, wrote, terminal)
}

/// Executable reject after cut/cancel: return notice in the response only.
/// Do not append/persist business turns on the (possibly cut) session.
pub(super) fn cancelled_turn_outcome(session: &mut Session, turns_checkpoint: usize) -> TurnOutcome {
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
