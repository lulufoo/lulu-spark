//! SSE assembly for OpenAI-compatible chat/completions streams.

use std::error::Error as StdError;
use std::io::{BufRead, BufReader, Read};

use serde_json::Value;

use super::{AssistantMessage, LlmCallOutcome, LlmError, ToolCall};
use crate::agent::session;

pub(super) fn assemble_sse(
    resp: impl Read,
    on_delta: &mut Option<&mut dyn FnMut(&str)>,
) -> LlmCallOutcome {
    let mut reader = BufReader::new(resp);
    let mut assembler = StreamAssembler::default();
    let mut data_buf = String::new();
    let mut failure: Option<LlmError> = None;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                failure = Some(map_read_error(error));
                break;
            }
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if data_buf.is_empty() {
                continue;
            }
            let payload = std::mem::take(&mut data_buf);
            if payload.trim() == "[DONE]" {
                break;
            }
            if let Err(error) = emit_if_noteworthy(&mut assembler, &payload, on_delta) {
                failure = Some(error);
                break;
            }
            continue;
        }
        if line.starts_with(':') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("data:") {
            let piece = rest.strip_prefix(' ').unwrap_or(rest);
            if piece.trim() == "[DONE]" {
                break;
            }
            if !data_buf.is_empty() {
                data_buf.push('\n');
            }
            data_buf.push_str(piece);
        }
    }
    if failure.is_none() && !data_buf.is_empty() && data_buf.trim() != "[DONE]" {
        if let Err(error) = emit_if_noteworthy(&mut assembler, &data_buf, on_delta) {
            failure = Some(error);
        }
    }
    // What arrived is always kept; whether it is acceptable is decided separately.
    let error = failure.or_else(|| assembler.classify_error());
    LlmCallOutcome {
        snapshot: Some(assembler.build_snapshot()),
        error,
    }
}

fn emit_if_noteworthy(
    assembler: &mut StreamAssembler,
    payload: &str,
    on_delta: &mut Option<&mut dyn FnMut(&str)>,
) -> Result<(), LlmError> {
    if apply_sse_payload(assembler, payload)? {
        if let Some(callback) = on_delta.as_mut() {
            callback(&assembler.hint());
        }
    }
    Ok(())
}

fn apply_sse_payload(assembler: &mut StreamAssembler, payload: &str) -> Result<bool, LlmError> {
    let parsed: Value = serde_json::from_str(payload)
        .map_err(|error| LlmError::InvalidResponse(format!("sse json: {error}")))?;
    if let Some(error) = parsed.get("error") {
        let msg = error
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("sse error");
        return Err(LlmError::InvalidResponse(msg.to_string()));
    }
    // Response-level fields live outside `choices`; usage-only chunks have empty `choices`.
    assembler.record_envelope(&parsed);
    let Some(choice) = parsed.pointer("/choices/0") else {
        return Ok(false);
    };
    if let Some(reason) = choice.get("finish_reason").and_then(|v| v.as_str()) {
        assembler.finish_reason = Some(reason.to_string());
    }
    let Some(delta) = choice.get("delta").or_else(|| choice.get("message")) else {
        return Ok(false);
    };
    Ok(assembler.apply_delta(delta))
}

fn map_read_error(error: std::io::Error) -> LlmError {
    if io_error_is_timeout(&error) {
        let _ = session::log_agent_error("LLM timeout reading stream");
        return LlmError::Timeout;
    }
    let _ = session::log_agent_error(&format!("LLM stream read error: {error}"));
    LlmError::Network(error.to_string())
}

fn io_error_is_timeout(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::TimedOut {
        return true;
    }
    let mut current: Option<&dyn StdError> = Some(error);
    while let Some(err) = current {
        let lower = err.to_string().to_ascii_lowercase();
        if lower.contains("timed out") || lower.contains("timeout") {
            return true;
        }
        current = err.source();
    }
    false
}

#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl PartialToolCall {
    fn is_blank(&self) -> bool {
        self.id.is_empty() && self.name.is_empty() && self.arguments.is_empty()
    }
}

#[derive(Default)]
struct StreamAssembler {
    content: String,
    reasoning: String,
    tool_calls: Vec<PartialToolCall>,
    finish_reason: Option<String>,
    model: Option<String>,
    usage: Option<Value>,
}

impl StreamAssembler {
    fn record_envelope(&mut self, chunk: &Value) {
        if let Some(model) = chunk.get("model").and_then(|v| v.as_str()) {
            if !model.is_empty() {
                self.model = Some(model.to_string());
            }
        }
        // `usage: null` is common on non-final chunks; only a real object counts.
        if let Some(usage) = chunk.get("usage").filter(|v| v.is_object()) {
            self.usage = Some(usage.clone());
        }
    }

    fn apply_delta(&mut self, delta: &Value) -> bool {
        let mut noteworthy = false;
        if let Some(piece) = delta.get("content").and_then(|v| v.as_str()) {
            if !piece.is_empty() {
                self.content.push_str(piece);
                noteworthy = true;
            }
        }
        if let Some(piece) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
            if !piece.is_empty() {
                self.reasoning.push_str(piece);
                noteworthy = true;
            }
        }
        if let Some(calls) = delta.get("tool_calls").and_then(|v| v.as_array()) {
            for call in calls {
                self.apply_tool_delta(call);
            }
            noteworthy = true;
        }
        noteworthy
    }

    fn apply_tool_delta(&mut self, call: &Value) {
        let slot = if let Some(index) = call.get("index").and_then(|v| v.as_u64()) {
            let index = index as usize;
            if self.tool_calls.len() <= index {
                self.tool_calls
                    .resize_with(index + 1, PartialToolCall::default);
            }
            &mut self.tool_calls[index]
        } else {
            self.tool_calls.push(PartialToolCall::default());
            self.tool_calls.last_mut().expect("just pushed")
        };
        if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
            if !id.is_empty() {
                slot.id = id.to_string();
            }
        }
        if let Some(name) = call.pointer("/function/name").and_then(|v| v.as_str()) {
            if !name.is_empty() {
                slot.name = name.to_string();
            }
        }
        match call.pointer("/function/arguments") {
            Some(Value::String(piece)) => slot.arguments.push_str(piece),
            Some(obj) if obj.is_object() => slot.arguments.push_str(&obj.to_string()),
            _ => {}
        }
    }

    fn hint(&self) -> String {
        if let Some(slot) = self
            .tool_calls
            .iter()
            .rev()
            .find(|slot| !slot.name.is_empty())
        {
            return format!("Preparing {}…", slot.name);
        }
        if self
            .tool_calls
            .iter()
            .any(|slot| !slot.id.is_empty() || !slot.arguments.is_empty())
        {
            return "Preparing…".into();
        }
        if !self.content.is_empty() {
            return format!("Receiving… {}", preview_tail(&self.content, 40));
        }
        if !self.reasoning.is_empty() {
            return "Thinking…".into();
        }
        "Receiving…".into()
    }

    /// Unconditional view of everything received so far; never fails, never normalises.
    fn build_snapshot(&self) -> AssistantMessage {
        AssistantMessage {
            content: non_empty(&self.content),
            tool_calls: self
                .tool_calls
                .iter()
                .filter(|slot| !slot.is_blank())
                .map(|slot| ToolCall {
                    id: slot.id.clone(),
                    name: slot.name.clone(),
                    arguments: slot.arguments.clone(),
                })
                .collect(),
            finish_reason: self.finish_reason.clone(),
            reasoning_content: non_empty(&self.reasoning),
            model: self.model.clone(),
            usage: self.usage.clone(),
        }
    }

    /// Decides whether what arrived is acceptable. Only this step may reject a response.
    fn classify_error(&self) -> Option<LlmError> {
        if self.finish_reason.as_deref() == Some("length") {
            let _ = session::log_agent_error("LLM finish_reason=length (truncated)");
            return Some(LlmError::Truncated);
        }
        let mut has_tool_call = false;
        for slot in self.tool_calls.iter().filter(|slot| !slot.is_blank()) {
            if slot.id.is_empty() || slot.name.is_empty() {
                return Some(LlmError::InvalidResponse("malformed tool_calls".into()));
            }
            has_tool_call = true;
        }
        if self.content.is_empty() && !has_tool_call && self.finish_reason.is_none() {
            return Some(LlmError::InvalidResponse("missing choices".into()));
        }
        None
    }
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_string())
}

fn preview_tail(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    format!(
        "…{}",
        text.chars().skip(count - max_chars).collect::<String>()
    )
}
