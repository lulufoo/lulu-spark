//! OpenAI-compatible streaming chat/completions client.

mod stream;

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

#[derive(Debug, Clone, Default)]
pub struct AssistantMessage {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    /// Raw upstream string; unknown values are kept as-is (vendors add their own).
    pub finish_reason: Option<String>,
    /// Model's reasoning (`delta.reasoning_content`); `None` when upstream sends none.
    pub reasoning_content: Option<String>,
    /// Response-level `model` as reported by upstream (may differ from the requested one).
    pub model: Option<String>,
    /// Response-level `usage` object, kept verbatim; `None` when upstream sends none.
    pub usage: Option<Value>,
}

/// Result of one streamed call, split into "what arrived" and "was it acceptable".
///
/// `snapshot` is whatever the stream delivered, even when the call is classified as an
/// error (truncated, malformed tool_calls, read failure mid-stream). It is `None` only
/// when no response body was ever read (config / connect / HTTP status errors).
#[derive(Debug, Clone)]
pub struct LlmCallOutcome {
    pub snapshot: Option<AssistantMessage>,
    pub error: Option<LlmError>,
}

impl LlmCallOutcome {
    fn failed(error: LlmError) -> Self {
        Self {
            snapshot: None,
            error: Some(error),
        }
    }

    /// Legacy view: the error decision wins; on success empty tool arguments become `{}`.
    pub fn into_result(self) -> Result<AssistantMessage, LlmError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let mut message = self
            .snapshot
            .ok_or_else(|| LlmError::InvalidResponse("missing response".into()))?;
        for call in &mut message.tool_calls {
            if call.arguments.is_empty() {
                call.arguments = "{}".into();
            }
        }
        Ok(message)
    }
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
    on_delta: Option<&mut dyn FnMut(&str)>,
) -> Result<AssistantMessage, LlmError> {
    chat_completions_traced(messages, tools, config, timeout, on_delta).into_result()
}

/// Same call as [`chat_completions_with_timeout`], but keeps the partial response
/// available when the call is classified as an error.
pub fn chat_completions_traced(
    messages: &[Value],
    tools: &[Value],
    config: &LlmConfig,
    timeout: Duration,
    mut on_delta: Option<&mut dyn FnMut(&str)>,
) -> LlmCallOutcome {
    if let Err(error) = validate_config(config) {
        return LlmCallOutcome::failed(error);
    }

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
    let client = match Client::builder()
        .timeout(timeout)
        .connect_timeout(timeout)
        .build()
    {
        Ok(client) => client,
        Err(e) => return LlmCallOutcome::failed(LlmError::Network(e.to_string())),
    };

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
                return LlmCallOutcome::failed(LlmError::Timeout);
            }
            let _ = session::log_agent_error(&format!("LLM network error: {e}"));
            return LlmCallOutcome::failed(LlmError::Network(e.to_string()));
        }
    };

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        let text = resp.text().unwrap_or_default();
        return LlmCallOutcome::failed(map_error_status(status, &text));
    }

    stream::assemble_sse(resp, &mut on_delta)
}

fn map_error_status(status: u16, text: &str) -> LlmError {
    let parsed: Value = serde_json::from_str(text).unwrap_or(json!({}));
    if status == 401 || status == 403 {
        let _ = session::log_agent_error(&format!("LLM auth failed status={status}"));
        return LlmError::Unauthorized;
    }
    if status == 429 {
        let _ = session::log_agent_error("LLM rate limited (429)");
        return LlmError::RateLimited;
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
            return LlmError::UnsupportedToolCalls;
        }
        let _ = session::log_agent_error(&format!("LLM bad request: {msg}"));
        return LlmError::BadRequest(msg.to_string());
    }
    if (500..600).contains(&status) {
        let _ = session::log_agent_error(&format!("LLM server error status={status}"));
        return LlmError::Server(status);
    }
    let _ = session::log_agent_error(&format!("LLM unexpected status={status}"));
    LlmError::InvalidResponse(format!("status {status}"))
}
