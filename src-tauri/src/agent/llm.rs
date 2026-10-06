//! OpenAI-compatible streaming chat/completions client.

use std::error::Error as StdError;
use std::io::{BufRead, BufReader, Read};
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{json, Value};

use crate::agent::session;
use crate::config::secrets;
use crate::config::settings;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    MissingConfig,
    Unauthorized,
    BadRequest(String),
    RateLimited,
    Server(u16),
    Timeout,
    Truncated,
    UnsupportedToolCalls,
    Network(String),
    InvalidResponse(String),
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct AssistantMessage {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: Option<String>,
}

pub fn load_llm_config() -> Result<LlmConfig, LlmError> {
    let settings = settings::load().map_err(|_| LlmError::MissingConfig)?;
    let Some(engine) = settings::normalize_engine_value(&settings.assistant_engine) else {
        return Err(LlmError::MissingConfig);
    };
    let api_key = secrets::get_secret()
        .map_err(|_| LlmError::MissingConfig)?
        .unwrap_or_default();
    let entry = settings::llm_entry_by_type(&settings.llm, engine)
        .filter(|entry| settings::is_supported_host_llm_entry(entry));
    let cfg = LlmConfig {
        api_key,
        base_url: entry.map(|e| e.base_url.clone()).unwrap_or_default(),
        model: entry.map(|e| e.model.clone()).unwrap_or_default(),
    };
    validate_config(&cfg)?;
    Ok(cfg)
}

fn validate_config(config: &LlmConfig) -> Result<(), LlmError> {
    if config.api_key.trim().is_empty()
        || config.base_url.trim().is_empty()
        || config.model.trim().is_empty()
    {
        return Err(LlmError::MissingConfig);
    }
    Ok(())
}

fn chat_url(base_url: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    // OpenAI-style …/v1；智谱 OpenAI 兼容 …/paas/v4（见 https://docs.bigmodel.cn/cn/guide/develop/openai/introduction）
    if base.ends_with("/v1") || base.ends_with("/v4") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

pub fn chat_completions(
    messages: &[Value],
    tools: &[Value],
    config: &LlmConfig,
) -> Result<AssistantMessage, LlmError> {
    chat_completions_with_timeout(messages, tools, config, DEFAULT_TIMEOUT, None)
}

pub fn chat_completions_with_timeout(
    messages: &[Value],
    tools: &[Value],
    config: &LlmConfig,
    timeout: Duration,
    mut on_delta: Option<&mut dyn FnMut(&str)>,
) -> Result<AssistantMessage, LlmError> {
    validate_config(config)?;

    let url = chat_url(&config.base_url);
    let mut body = json!({
        "model": config.model,
        "messages": messages,
        "stream": true,
    });
    // Empty tools = omit tools + tool_choice (Host business path / tools=[] semantics).
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools.to_vec());
        body["tool_choice"] = json!("auto");
    }

    // Blocking reqwest applies this duration to connect/TTFB and to each body
    // read, resetting after a successful read — idle timeout for SSE chunks.
    let client = Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()
        .map_err(|e| LlmError::Network(e.to_string()))?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key.trim()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send();

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            if e.is_timeout() {
                let _ = session::log_agent_error(&format!("LLM timeout calling {url}"));
                return Err(LlmError::Timeout);
            }
            let _ = session::log_agent_error(&format!("LLM network error: {e}"));
            return Err(LlmError::Network(e.to_string()));
        }
    };

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let text = resp.text().unwrap_or_default();
        return map_error_status(status, &text);
    }

    assemble_sse(resp, &mut on_delta)
}

fn map_error_status(status: u16, text: &str) -> Result<AssistantMessage, LlmError> {
    let parsed: Value = serde_json::from_str(text).unwrap_or(json!({}));
    if status == 401 || status == 403 {
        let _ = session::log_agent_error(&format!("LLM auth failed status={status}"));
        return Err(LlmError::Unauthorized);
    }
    if status == 429 {
        let _ = session::log_agent_error("LLM rate limited (429)");
        return Err(LlmError::RateLimited);
    }
    if status == 400 {
        let msg = parsed
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or(text);
        let lower = msg.to_ascii_lowercase();
        if lower.contains("tool_calls")
            || lower.contains("tools / tool")
            || (lower.contains("tools") && lower.contains("not supported"))
            || (lower.contains("tool") && lower.contains("not supported"))
        {
            let _ = session::log_agent_error("LLM upstream does not support tool_calls");
            return Err(LlmError::UnsupportedToolCalls);
        }
        let _ = session::log_agent_error(&format!("LLM bad request: {msg}"));
        return Err(LlmError::BadRequest(msg.to_string()));
    }
    if (500..600).contains(&status) {
        let _ = session::log_agent_error(&format!("LLM server error status={status}"));
        return Err(LlmError::Server(status));
    }
    let _ = session::log_agent_error(&format!("LLM unexpected status={status}"));
    Err(LlmError::InvalidResponse(format!("status {status}")))
}

fn assemble_sse(
    resp: impl Read,
    on_delta: &mut Option<&mut dyn FnMut(&str)>,
) -> Result<AssistantMessage, LlmError> {
    let mut reader = BufReader::new(resp);
    let mut assembler = StreamAssembler::default();
    let mut data_buf = String::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => return Err(map_read_error(error)),
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
            emit_if_noteworthy(&mut assembler, &payload, on_delta)?;
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
    if !data_buf.is_empty() && data_buf.trim() != "[DONE]" {
        emit_if_noteworthy(&mut assembler, &data_buf, on_delta)?;
    }
    assembler.finish()
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

#[derive(Default)]
struct StreamAssembler {
    content: String,
    reasoning: String,
    tool_calls: Vec<PartialToolCall>,
    finish_reason: Option<String>,
}

impl StreamAssembler {
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

    fn finish(self) -> Result<AssistantMessage, LlmError> {
        if self.finish_reason.as_deref() == Some("length") {
            let _ = session::log_agent_error("LLM finish_reason=length (truncated)");
            return Err(LlmError::Truncated);
        }
        let mut tool_calls = Vec::new();
        for slot in self.tool_calls {
            if slot.id.is_empty() && slot.name.is_empty() && slot.arguments.is_empty() {
                continue;
            }
            if slot.id.is_empty() || slot.name.is_empty() {
                return Err(LlmError::InvalidResponse("malformed tool_calls".into()));
            }
            tool_calls.push(ToolCall {
                id: slot.id,
                name: slot.name,
                arguments: if slot.arguments.is_empty() {
                    "{}".into()
                } else {
                    slot.arguments
                },
            });
        }
        if self.content.is_empty() && tool_calls.is_empty() && self.finish_reason.is_none() {
            return Err(LlmError::InvalidResponse("missing choices".into()));
        }
        Ok(AssistantMessage {
            content: if self.content.is_empty() {
                None
            } else {
                Some(self.content)
            },
            tool_calls,
            finish_reason: self.finish_reason,
        })
    }
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
