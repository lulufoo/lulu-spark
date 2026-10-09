//! Compress active model turns into one `summaries` row before the next user send.

use serde_json::{json, Value};

use crate::agent::llm::{self, LlmConfig};
use crate::agent::session::{self, Session, Step};

use super::count::count_tokens;
use super::render::render_request;
use super::window::{round_percent, window_tokens};

pub const COMPRESS_THRESHOLD_PERCENT: i64 = 70;

pub const SUMMARY_SYSTEM_PROMPT: &str =
    "把下面的对话压成一段给后续主请求用的摘要。保留已做的决定、结论和未完成事项。不要向用户说话。";

/// True when the last stored prompt already occupies at least 70% of the window.
pub fn should_compress(tokens: i64, window: u64) -> bool {
    if tokens < 0 {
        return false;
    }
    round_percent(tokens as u64, window)
        .is_some_and(|percent| percent >= COMPRESS_THRESHOLD_PERCENT)
}

/// System prompt plus one user message that holds the packed transcript.
pub fn pack_summary_messages(steps: &[Step]) -> Vec<Value> {
    vec![
        json!({
            "role": "system",
            "content": SUMMARY_SYSTEM_PROMPT,
        }),
        json!({
            "role": "user",
            "content": format_transcript(steps),
        }),
    ]
}

/// Shrink the summary pack until it is below the window.
/// Tool results go first, then other step text. `None` only when even an
/// emptied pack still reaches the window.
pub fn fit_summary_messages(steps: &[Step], window: u64) -> Option<Vec<Value>> {
    if window == 0 {
        return None;
    }
    let mut steps = steps.to_vec();
    loop {
        let messages = pack_summary_messages(&steps);
        let text = render_request(&messages, &[]);
        let tokens = count_tokens(&text).ok()? as u64;
        if tokens < window {
            return Some(messages);
        }
        if !shrink_longest_pack_field(&mut steps) {
            return None;
        }
    }
}

/// If the last snapshot is at least 70% full, replace every active turn with one summary.
/// Failures leave the session unchanged. Does not write the context snapshot.
pub fn maybe_compress(session: &mut Session, config: &LlmConfig) {
    let Ok(Some(tokens)) = session::load_last_prompt_tokens(&session.session_id) else {
        return;
    };
    let window = window_tokens(&config.model);
    if !should_compress(tokens, window) || session.steps.is_empty() {
        return;
    }
    let Ok(Some((from_seq, to_seq))) = session::active_turn_seq_range(&session.session_id) else {
        return;
    };
    let Some(messages) = fit_summary_messages(&session.steps, window) else {
        return;
    };
    let ctx = llm::CallContext {
        session_id: session.session_id.clone(),
        trace_id: None,
        purpose: "summary",
        round: 0,
        step_index: session.steps.len(),
    };
    let Ok(reply) = llm::chat_completions_recorded(
        &messages,
        &[],
        config,
        llm::DEFAULT_TIMEOUT,
        None,
        Some(&ctx),
    )
    .into_result() else {
        return;
    };
    let Some(body) = reply
        .content
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
    else {
        return;
    };
    if session::replace_turns_with_summary(&session.session_id, from_seq, to_seq, &body).is_err()
    {
        return;
    }
    session.steps = session::load_session(&session.session_id)
        .map(|reloaded| reloaded.steps)
        .unwrap_or_default();
}

fn format_transcript(steps: &[Step]) -> String {
    let mut out = String::new();
    for step in steps {
        if !out.is_empty() {
            out.push('\n');
        }
        match step.role.as_str() {
            "tool" => {
                out.push_str("tool");
                if let Some(name) = &step.name {
                    out.push(' ');
                    out.push_str(name);
                }
                out.push('\n');
                out.push_str(step.content.as_deref().unwrap_or(""));
            }
            _ if step.tool_calls.is_some() => {
                out.push_str("assistant\n");
                if let Some(content) = &step.content {
                    if !content.is_empty() {
                        out.push_str(content);
                        out.push('\n');
                    }
                }
                if let Some(calls) = &step.tool_calls {
                    out.push_str(&calls.to_string());
                }
            }
            _ => {
                out.push_str(&step.role);
                out.push('\n');
                out.push_str(step.content.as_deref().unwrap_or(""));
            }
        }
    }
    out
}

fn shrink_longest_pack_field(steps: &mut [Step]) -> bool {
    if shrink_longest_where(steps, |step| step.role == "tool") {
        return true;
    }
    shrink_longest_where(steps, |_| true)
}

fn shrink_longest_where(steps: &mut [Step], keep: impl Fn(&Step) -> bool) -> bool {
    let Some(index) = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| keep(step) && shrinkable_len(step) > 0)
        .max_by_key(|(_, step)| shrinkable_len(step))
        .map(|(index, _)| index)
    else {
        return false;
    };
    shrink_step(&mut steps[index])
}

fn shrinkable_len(step: &Step) -> usize {
    let content = step.content.as_deref().map(str::len).unwrap_or(0);
    let calls = step
        .tool_calls
        .as_ref()
        .map(|calls| match calls {
            Value::String(text) => text.len(),
            other => other.to_string().len(),
        })
        .unwrap_or(0);
    content + calls
}

fn shrink_step(step: &mut Step) -> bool {
    if let Some(content) = step.content.as_mut() {
        if !content.is_empty() {
            let keep = content.chars().count() / 2;
            *content = content.chars().take(keep).collect();
            return true;
        }
    }
    let Some(calls) = step.tool_calls.take() else {
        return false;
    };
    let text = match calls {
        Value::String(text) => text,
        other => other.to_string(),
    };
    if text.is_empty() {
        return false;
    }
    let keep = text.chars().count() / 2;
    step.tool_calls = Some(Value::String(text.chars().take(keep).collect()));
    true
}
