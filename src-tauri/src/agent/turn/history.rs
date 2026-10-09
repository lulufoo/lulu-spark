//! History caps, LLM message build, persist, and cancelled-turn outcome.

use serde_json::{json, Value};

use crate::agent::llm::LlmError;
use crate::agent::session::{self, Session, Step};

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
        LlmError::ThinkingExhausted => {
            "模型的思考占满了输出预算，没有给出回复，请缩短请求或稍后重试。".into()
        }
        LlmError::ContentFiltered => "回复被上游的安全审核拦截，请调整请求后重试。".into(),
        LlmError::InferenceFailed => "模型推理出现异常，请稍后重试。".into(),
        LlmError::ContextExceeded => {
            "对话内容超出了模型的上下文窗口，请缩短请求或开启新会话。".into()
        }
        LlmError::UnknownFinish(reason) => {
            format!("模型以未知原因（{reason}）结束，没有给出回复，未执行任何写入。")
        }
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
pub(crate) fn cancelled_turn_outcome(session: &mut Session, steps_checkpoint: usize) -> TurnOutcome {
    if session.steps.len() != steps_checkpoint {
        session.steps.truncate(steps_checkpoint);
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
