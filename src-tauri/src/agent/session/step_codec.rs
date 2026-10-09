//! One `model_steps` row <-> one `Step` (kept one-to-one, nothing is merged).

use serde_json::{json, Value};

use super::step_clock::StepClock;
use super::types::Step;

/// The columns of one `model_steps` row.
pub(super) struct StepRow {
    pub kind: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub finish_reason: Option<String>,
    pub model: Option<String>,
    /// JSON text of the upstream `usage` object.
    pub usage: Option<String>,
    pub reasoning_content: Option<String>,
    /// Unix milliseconds; see `StepClock`.
    pub create_ts: Option<i64>,
    pub start_ts: Option<i64>,
    pub end_ts: Option<i64>,
}

pub(super) fn encode_step(step: &Step) -> StepRow {
    let (kind, content, tool_call_id, tool_name) = encode_base(step);
    StepRow {
        kind,
        content,
        tool_call_id,
        tool_name,
        finish_reason: step.finish_reason.clone(),
        model: step.model.clone(),
        usage: step.usage.as_ref().map(Value::to_string),
        reasoning_content: step.reasoning_content.clone(),
        create_ts: step.clock.create_ts,
        start_ts: step.clock.start_ts,
        end_ts: step.clock.end_ts,
    }
}

fn encode_base(step: &Step) -> (String, String, Option<String>, Option<String>) {
    let text = step.content.clone().unwrap_or_default();
    match step.role.as_str() {
        "tool" => ("tool_result".into(), text, step.tool_call_id.clone(), step.name.clone()),
        "user" => ("user".into(), text, None, None),
        _ if step.tool_calls.is_some() => (
            "tool_call".into(),
            json!({ "content": step.content, "tool_calls": step.tool_calls }).to_string(),
            step.tool_call_id.clone(),
            step.name.clone(),
        ),
        _ => ("assistant".into(), text, step.tool_call_id.clone(), step.name.clone()),
    }
}

pub(super) fn decode_step(row: StepRow) -> Step {
    let mut step = decode_base(&row.kind, &row.content, row.tool_call_id, row.tool_name);
    step.finish_reason = row.finish_reason;
    step.model = row.model;
    step.usage = row.usage.and_then(|text| serde_json::from_str(&text).ok());
    step.reasoning_content = row.reasoning_content;
    step.clock = StepClock {
        create_ts: row.create_ts,
        start_ts: row.start_ts,
        end_ts: row.end_ts,
    };
    step
}

fn decode_base(
    kind: &str,
    content: &str,
    tool_call_id: Option<String>,
    tool_name: Option<String>,
) -> Step {
    if kind == "tool_call" {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(content) {
            let text = match map.get("content") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Null) | None => None,
                Some(other) => Some(other.to_string()),
            };
            return Step {
                role: "assistant".into(),
                content: text,
                tool_call_id,
                tool_calls: map.get("tool_calls").cloned(),
                name: tool_name,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
                clock: StepClock::default(),
            };
        }
    }
    Step {
        role: match kind {
            "user" => "user",
            "tool_result" => "tool",
            _ => "assistant",
        }
        .into(),
        content: (!content.is_empty()).then(|| content.to_string()),
        tool_call_id,
        tool_calls: None,
        name: tool_name,
        finish_reason: None,
        model: None,
        usage: None,
        reasoning_content: None,
        clock: StepClock::default(),
    }
}
