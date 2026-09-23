//! Remember the last model prompt of a user turn, and turn it into a percentage.

use serde_json::Value;

use crate::agent::llm;
use crate::agent::session;

use super::count::count_tokens;
use super::render::render_request;
use super::window::{round_percent, window_tokens};

/// Count the prompt that is about to be sent, and keep it as this session's latest.
pub fn record_sent_prompt(session_id: &str, messages: &[Value], tools: &[Value]) {
    let Ok(tokens) = count_tokens(&render_request(messages, tools)) else {
        return;
    };
    let Ok(tokens) = i64::try_from(tokens) else {
        return;
    };
    let _ = session::store_last_prompt_tokens(session_id, tokens);
}

/// Percentage for the last model prompt sent in the current session.
/// `None` when there is no session, the host is unbound, no prompt has been
/// sent, or the model has no window row.
pub fn current_context_percent() -> Option<i64> {
    let live = session::live_context_owner();
    let session_id = live.current_session_id.clone().unwrap_or_default();
    if session_id.is_empty() || live.current_binding.is_none() {
        return None;
    }
    let tokens = session::load_last_prompt_tokens(&session_id).ok()??;
    if tokens < 0 {
        return None;
    }
    let model = llm::load_llm_config().ok()?.model;
    let window = window_tokens(&model)?;
    round_percent(tokens as u64, window)
}
