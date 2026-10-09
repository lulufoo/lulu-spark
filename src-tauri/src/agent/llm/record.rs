//! One file per LLM request: `{sessions}/{session_id}/llm-calls/{NNNN}.json`.
//!
//! Policy: this is a deliberate exception to "prompt/reply bodies never go to logs".
//! That rule (see `diagnostics.rs`) covers `assistant-diagnostic.jsonl` only. These files
//! hold the full request and what the response delivered, stay out of every model-readable
//! directory, and are deleted together with the session. HTTP headers are never recorded,
//! and every string goes through `redact_secrets`.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use chrono::Utc;
use serde_json::{json, Value};

use super::{AssistantMessage, LlmCallOutcome, LlmError};
use crate::agent::session;

const ERROR_BODY_LIMIT: usize = 8 * 1024;

/// Who is asking, so a record can be tied back to a Turn and a position in the session.
#[derive(Debug, Clone)]
pub struct CallContext {
    pub session_id: String,
    /// Same id as the `assistant-diagnostic.jsonl` events of the Turn; `None` outside a Turn.
    pub trace_id: Option<String>,
    /// `turn` for the ReAct loop, `summary` for history compression.
    pub purpose: &'static str,
    /// 1-based tool-loop round inside the Turn; 0 when not part of the loop.
    pub round: usize,
    /// Index the response Step takes in `session.steps` (the Turn id does not exist yet).
    pub step_index: usize,
}

/// Timings and HTTP facts collected while one request runs.
pub(super) struct CallTrace {
    started: Instant,
    started_at: String,
    first_byte_ms: Option<u64>,
    first_reasoning_ms: Option<u64>,
    first_content_ms: Option<u64>,
    status: Option<u16>,
    error_body: Option<String>,
}

impl CallTrace {
    pub(super) fn start() -> Self {
        Self {
            started: Instant::now(),
            started_at: Utc::now().to_rfc3339(),
            first_byte_ms: None,
            first_reasoning_ms: None,
            first_content_ms: None,
            status: None,
            error_body: None,
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    /// Response headers arrived.
    pub(super) fn mark_headers(&mut self, status: u16) {
        let now = self.elapsed_ms();
        self.first_byte_ms.get_or_insert(now);
        self.status = Some(status);
    }

    pub(super) fn set_error_body(&mut self, text: &str) {
        let cut: String = text.chars().take(ERROR_BODY_LIMIT).collect();
        self.error_body = Some(cut);
    }

    /// Called after every stream event with what has been assembled so far.
    pub(super) fn observe(&mut self, has_reasoning: bool, has_content: bool) {
        let now = self.elapsed_ms();
        if has_reasoning {
            self.first_reasoning_ms.get_or_insert(now);
        }
        if has_content {
            self.first_content_ms.get_or_insert(now);
        }
    }
}

/// Never fails the call: a write problem is logged and dropped.
pub(super) fn write(
    ctx: &CallContext,
    url: &str,
    body: &Value,
    trace: &CallTrace,
    outcome: &LlmCallOutcome,
) {
    let mut record = build(ctx, url, body, trace, outcome);
    redact_strings(&mut record);
    let result = session::session_llm_calls_dir(&ctx.session_id).and_then(|dir| {
        let text = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
        write_next(&dir, text.as_bytes())
    });
    if let Err(error) = result {
        let _ = session::log_agent_error(&format!("llm-call record not written: {error}"));
    }
}

fn build(
    ctx: &CallContext,
    url: &str,
    body: &Value,
    trace: &CallTrace,
    outcome: &LlmCallOutcome,
) -> Value {
    json!({
        "version": 1,
        "session_id": ctx.session_id,
        "trace_id": ctx.trace_id,
        "purpose": ctx.purpose,
        "round": ctx.round,
        "step_index": ctx.step_index,
        "started_at": trace.started_at,
        "request": { "url": url, "body": body },
        "response": {
            "http_status": trace.status,
            "message": outcome.snapshot.as_ref().map(message_json),
            "error_body": trace.error_body,
        },
        "timing_ms": {
            "first_byte": trace.first_byte_ms,
            "first_reasoning": trace.first_reasoning_ms,
            "first_content": trace.first_content_ms,
            "total": trace.elapsed_ms(),
        },
        "outcome": {
            "category": category(outcome),
            "error": outcome.error.as_ref().map(|error| format!("{error:?}")),
        },
    })
}

fn message_json(message: &AssistantMessage) -> Value {
    json!({
        "content": message.content,
        "reasoning_content": message.reasoning_content,
        "tool_calls": message.tool_calls.iter().map(|call| json!({
            "id": call.id,
            "name": call.name,
            "arguments": call.arguments,
        })).collect::<Vec<_>>(),
        "finish_reason": message.finish_reason,
        "model": message.model,
        "usage": message.usage,
    })
}

/// ok / truncated / empty / timeout / status / error.
fn category(outcome: &LlmCallOutcome) -> &'static str {
    match &outcome.error {
        None => match &outcome.snapshot {
            Some(m) if m.content.is_none() && m.tool_calls.is_empty() => "empty",
            _ => "ok",
        },
        Some(LlmError::Truncated | LlmError::ThinkingExhausted) => "truncated",
        Some(LlmError::Timeout) => "timeout",
        Some(
            LlmError::Unauthorized
            | LlmError::RateLimited
            | LlmError::Server(_)
            | LlmError::BadRequest(_)
            | LlmError::UnsupportedToolCalls,
        ) => "status",
        Some(_) => "error",
    }
}

fn redact_strings(value: &mut Value) {
    match value {
        Value::String(text) => *text = session::redact_secrets(text),
        Value::Array(items) => items.iter_mut().for_each(redact_strings),
        Value::Object(map) => map.values_mut().for_each(redact_strings),
        _ => {}
    }
}

/// Writes `{N+1:04}.json`; `create_new` keeps two writers from sharing a number.
fn write_next(dir: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut next = highest_number(dir) + 1;
    loop {
        let path = dir.join(format!("{next:04}.json"));
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => return file.write_all(bytes).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => next += 1,
            Err(e) => return Err(e.to_string()),
        }
    }
}

fn highest_number(dir: &Path) -> u32 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "json" {
                return None;
            }
            path.file_stem()?.to_str()?.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0)
}
