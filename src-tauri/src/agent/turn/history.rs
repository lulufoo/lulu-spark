//! History caps, LLM message build, persist, and cancelled-turn outcome.

use serde_json::{json, Value};

use crate::agent::language::{describe_cancelled, describe_llm_error, Language};
use crate::agent::llm::LlmError;
use crate::agent::session::{self, Session, Step, EXTRA_APPEND_CONTENT};

use super::types::{Terminal, TurnOutcome, EVENT_TURN_COMPLETED, MAX_HISTORY_MESSAGES, MAX_USER_TURNS};

pub(super) fn is_clarify_text(content: &str) -> bool {
    let t = content.trim();
    t.contains('？') || t.contains('?')
}

/// `echo_reasoning` is true only for Steps after the last `user` Step: the same
/// Turn's tool loop. Older Turns are sent without their thinking.
fn step_to_message(step: &Step, echo_reasoning: bool) -> Value {
    let mut m = json!({ "role": step.role });
    if let (true, Some(thinking)) = (echo_reasoning, &step.reasoning_content) {
        m["reasoning_content"] = json!(thinking);
    }
    if let Some(c) = &step.content {
        m["content"] = json!(c);
    } else if step.tool_calls.is_none() {
        m["content"] = json!("");
    }
    if let Some(id) = &step.tool_call_id {
        m["tool_call_id"] = json!(id);
    }
    if let Some(name) = &step.name {
        m["name"] = json!(name);
    }
    if let Some(tc) = &step.tool_calls {
        m["tool_calls"] = tc.clone();
    }
    m
}

/// Drop oldest complete user rounds until dual hard caps hold.
pub fn truncate_steps(steps: &[Step]) -> Vec<Step> {
    let mut kept = steps.to_vec();
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

pub fn build_llm_messages_from_steps(steps: &[Step], system_prompt: &str) -> Vec<Value> {
    build_llm_messages(steps, system_prompt, &[])
}

pub fn build_llm_messages(steps: &[Step], system_prompt: &str, summaries: &[String]) -> Vec<Value> {
    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt,
    })];
    for body in summaries {
        if body.trim().is_empty() {
            continue;
        }
        messages.push(json!({
            "role": "user",
            "content": body,
        }));
    }
    let kept = truncate_steps(steps);
    let last_user = kept.iter().rposition(|step| step.role == "user");
    for (index, step) in kept.iter().enumerate() {
        let echo_reasoning = last_user.map_or(true, |user| index > user);
        messages.push(step_to_message(step, echo_reasoning));
        if let Some(text) = step.extra.get(EXTRA_APPEND_CONTENT).and_then(Value::as_str) {
            messages.push(json!({ "role": "user", "content": text }));
        }
    }
    messages
}

pub(crate) fn prompt_text_from_binding(prompt: &Value) -> String {
    match prompt {
        Value::String(s) => s.clone(),
        other => other
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| other.to_string()),
    }
}

pub fn map_llm_error(err: &LlmError, language: Language) -> TurnOutcome {
    let reply_text = describe_llm_error(language, err);
    TurnOutcome {
        reply_text,
        terminal: Terminal::Error,
        wrote: false,
    }
}

pub fn persist(session: &Session) {
    persist_thinking(session, None);
}

pub fn persist_thinking(session: &Session, thinking_ms: Option<u64>) {
    let _ = session::save_session(session);
    if let Some(ms) = thinking_ms {
        let _ = session::store_turn_thinking_ms(&session.session_id, ms);
    }
}

pub fn turn_completed_emit(session_id: &str, wrote: bool, terminal: &str) -> Value {
    emit_payload(session_id, wrote, terminal)
}

/// Executable reject after cut/cancel: return notice in the response only.
/// Do not append/persist business turns on the (possibly cut) session.
pub(crate) fn cancelled_turn_outcome(session: &mut Session, steps_checkpoint: usize) -> TurnOutcome {
    if session.steps.len() != steps_checkpoint {
        session.steps.truncate(steps_checkpoint);
        persist(session);
    }
    TurnOutcome {
        reply_text: describe_cancelled(Language::current()).to_string(),
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
