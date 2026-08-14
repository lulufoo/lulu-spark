//! OpenAI-compatible non-streaming chat/completions client.

use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::{json, Value};

use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings;
use crate::services::agent::session;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

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
    if !settings.assistant_engine.trim().eq_ignore_ascii_case("host") {
        return Err(LlmError::MissingConfig);
    }
    let api_key = secrets::get_secret(KEY_LLM_API_KEY)
        .map_err(|_| LlmError::MissingConfig)?
        .unwrap_or_default();
    let entry = settings::llm_entry_by_type(&settings.llm, "host");
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
    chat_completions_with_timeout(messages, tools, config, DEFAULT_TIMEOUT)
}

pub fn chat_completions_with_timeout(
    messages: &[Value],
    tools: &[Value],
    config: &LlmConfig,
    timeout: Duration,
) -> Result<AssistantMessage, LlmError> {
    validate_config(config)?;

    let url = chat_url(&config.base_url);
    let mut body = json!({
        "model": config.model,
        "messages": messages,
        "stream": false,
    });
    // Empty tools = omit tools + tool_choice (Host business path / tools=[] semantics).
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools.to_vec());
        body["tool_choice"] = json!("auto");
    }

    let client = Client::builder()
        .timeout(timeout)
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
    let text = resp.text().unwrap_or_default();
    let parsed: Value = serde_json::from_str(&text).unwrap_or(json!({}));

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
            .unwrap_or(text.as_str());
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
    if !(200..300).contains(&status) {
        let _ = session::log_agent_error(&format!("LLM unexpected status={status}"));
        return Err(LlmError::InvalidResponse(format!("status {status}")));
    }

    let choice = parsed
        .pointer("/choices/0")
        .ok_or_else(|| LlmError::InvalidResponse("missing choices".into()))?;
    let finish_reason = choice
        .get("finish_reason")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if finish_reason.as_deref() == Some("length") {
        let _ = session::log_agent_error("LLM finish_reason=length (truncated)");
        return Err(LlmError::Truncated);
    }

    let message = choice
        .get("message")
        .ok_or_else(|| LlmError::InvalidResponse("missing message".into()))?;

    let content = message
        .get("content")
        .and_then(|v| {
            if v.is_null() {
                None
            } else {
                v.as_str().map(|s| s.to_string())
            }
        });

    let mut tool_calls = Vec::new();
    if let Some(arr) = message.get("tool_calls").and_then(|v| v.as_array()) {
        for tc in arr {
            let id = tc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let name = tc
                .pointer("/function/name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let arguments = tc
                .pointer("/function/arguments")
                .and_then(|v| v.as_str())
                .unwrap_or("{}")
                .to_string();
            if id.is_empty() || name.is_empty() {
                return Err(LlmError::InvalidResponse("malformed tool_calls".into()));
            }
            tool_calls.push(ToolCall {
                id,
                name,
                arguments,
            });
        }
    }

    Ok(AssistantMessage {
        content,
        tool_calls,
        finish_reason,
    })
}
