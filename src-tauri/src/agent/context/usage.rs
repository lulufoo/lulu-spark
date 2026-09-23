//! Remember the last model prompt of a user turn, and turn it into a percentage.

use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::agent::llm;
use crate::agent::session;

use super::count::{count_spans, count_tokens};
use super::render::render_marked;
use super::window::{round_percent, window_tokens};

const CATEGORY_ORDER: [&str; 5] = ["system_prompt", "tools", "mcp", "conversation", "other"];

pub struct ContextCategory {
    pub id: String,
    pub tokens: i64,
}

pub struct ContextUsage {
    pub percent: i64,
    pub total_tokens: i64,
    pub window_tokens: u64,
    pub categories: Vec<ContextCategory>,
}

/// Count the prompt that is about to be sent, and keep it as this session's latest.
/// `mcp_names` selects which tool definitions are the MCP category. Host tool
/// definitions stay in Tools. The system message, the conversation, and the
/// template remainder are the other categories this prompt can support.
pub fn record_sent_prompt(
    session_id: &str,
    messages: &[Value],
    tools: &[Value],
    mcp_names: &BTreeSet<String>,
) {
    let (text, spans) = render_marked(messages, tools, mcp_names);
    let Ok(categories) = count_spans(&text, &spans) else {
        return;
    };
    let Ok(total) = count_tokens(&text) else {
        return;
    };
    let Ok(total) = i64::try_from(total) else {
        return;
    };
    let breakdown = json!(categories
        .iter()
        .map(|(id, tokens)| json!({ "id": id, "tokens": tokens }))
        .collect::<Vec<_>>())
    .to_string();
    let _ = session::store_last_prompt(session_id, total, &breakdown);
}

/// Percentage for the last model prompt sent in the current session.
/// `None` when there is no session, the host is unbound, no prompt has been
/// sent, or the model has no window row.
pub fn current_context_percent() -> Option<i64> {
    current_context_usage().map(|usage| usage.percent)
}

/// Last sent prompt, split into the categories that prompt actually contains.
pub fn current_context_usage() -> Option<ContextUsage> {
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
    let percent = round_percent(tokens as u64, window)?;
    let categories = session::load_last_prompt_breakdown(&session_id)
        .ok()
        .flatten()
        .map(|raw| parse_breakdown(&raw))
        .unwrap_or_default();
    Some(ContextUsage {
        percent,
        total_tokens: tokens,
        window_tokens: window,
        categories,
    })
}

fn parse_breakdown(raw: &str) -> Vec<ContextCategory> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    let Some(rows) = value.as_array() else {
        return Vec::new();
    };
    let mut by_id = Vec::new();
    for id in CATEGORY_ORDER {
        let Some(row) = rows.iter().find(|row| row.get("id").and_then(Value::as_str) == Some(id)) else {
            continue;
        };
        let Some(tokens) = row.get("tokens").and_then(Value::as_u64) else {
            continue;
        };
        if tokens == 0 {
            continue;
        }
        let Ok(tokens) = i64::try_from(tokens) else {
            continue;
        };
        by_id.push(ContextCategory {
            id: id.to_string(),
            tokens,
        });
    }
    by_id
}
