//! Compress active model turns into one `summaries` row before the next user send.

use serde_json::{json, Value};

use crate::agent::llm::{self, LlmConfig};
use crate::agent::session::{self, Session, Turn};

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
pub fn pack_summary_messages(turns: &[Turn]) -> Vec<Value> {
    vec![
        json!({
            "role": "system",
            "content": SUMMARY_SYSTEM_PROMPT,
        }),
        json!({
            "role": "user",
            "content": format_transcript(turns),
        }),
    ]
}

/// Shrink the summary pack until it is below the window.
/// Tool results go first, then other step text. `None` only when even an
/// emptied pack still reaches the window.
pub fn fit_summary_messages(turns: &[Turn], window: u64) -> Option<Vec<Value>> {
    if window == 0 {
        return None;
    }
    let mut turns = turns.to_vec();
    loop {
        let messages = pack_summary_messages(&turns);
        let text = render_request(&messages, &[]);
        let tokens = count_tokens(&text).ok()? as u64;
        if tokens < window {
            return Some(messages);
        }
        if !shrink_longest_pack_field(&mut turns) {
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
    if !should_compress(tokens, window) || session.turns.is_empty() {
        return;
    }
    let Ok(Some((from_seq, to_seq))) = session::active_turn_seq_range(&session.session_id) else {
        return;
    };
    let Some(messages) = fit_summary_messages(&session.turns, window) else {
        return;
    };
    let Ok(reply) = llm::chat_completions_with_timeout(
        &messages,
        &[],
        config,
        llm::DEFAULT_TIMEOUT,
        None,
    ) else {
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
    session.turns = session::load_session(&session.session_id)
        .map(|reloaded| reloaded.turns)
        .unwrap_or_default();
}

fn format_transcript(turns: &[Turn]) -> String {
    let mut out = String::new();
    for turn in turns {
        if !out.is_empty() {
            out.push('\n');
        }
        match turn.role.as_str() {
            "tool" => {
                out.push_str("tool");
                if let Some(name) = &turn.name {
                    out.push(' ');
                    out.push_str(name);
                }
                out.push('\n');
                out.push_str(turn.content.as_deref().unwrap_or(""));
            }
            _ if turn.tool_calls.is_some() => {
                out.push_str("assistant\n");
                if let Some(content) = &turn.content {
                    if !content.is_empty() {
                        out.push_str(content);
                        out.push('\n');
                    }
                }
                if let Some(calls) = &turn.tool_calls {
                    out.push_str(&calls.to_string());
                }
            }
            _ => {
                out.push_str(&turn.role);
                out.push('\n');
                out.push_str(turn.content.as_deref().unwrap_or(""));
            }
        }
    }
    out
}

fn shrink_longest_pack_field(turns: &mut [Turn]) -> bool {
    if shrink_longest_where(turns, |turn| turn.role == "tool") {
        return true;
    }
    shrink_longest_where(turns, |_| true)
}

fn shrink_longest_where(turns: &mut [Turn], keep: impl Fn(&Turn) -> bool) -> bool {
    let Some(index) = turns
        .iter()
        .enumerate()
        .filter(|(_, turn)| keep(turn) && shrinkable_len(turn) > 0)
        .max_by_key(|(_, turn)| shrinkable_len(turn))
        .map(|(index, _)| index)
    else {
        return false;
    };
    shrink_turn(&mut turns[index])
}

fn shrinkable_len(turn: &Turn) -> usize {
    let content = turn.content.as_deref().map(str::len).unwrap_or(0);
    let calls = turn
        .tool_calls
        .as_ref()
        .map(|calls| match calls {
            Value::String(text) => text.len(),
            other => other.to_string().len(),
        })
        .unwrap_or(0);
    content + calls
}

fn shrink_turn(turn: &mut Turn) -> bool {
    if let Some(content) = turn.content.as_mut() {
        if !content.is_empty() {
            let keep = content.chars().count() / 2;
            *content = content.chars().take(keep).collect();
            return true;
        }
    }
    let Some(calls) = turn.tool_calls.take() else {
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
    turn.tool_calls = Some(Value::String(text.chars().take(keep).collect()));
    true
}
