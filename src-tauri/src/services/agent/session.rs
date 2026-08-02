//! Session persistence under `{cache_dir}/agent/sessions/`.

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::paths;
use crate::services::id::random_hex12;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Turn {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Generic Binding Contract config surface (A1 opaque handles): tools + prompt + callbacks.
/// Not keyed by business IDs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    pub tools: Value,
    pub prompt: Value,
    pub callbacks: Value,
}

/// Read-only Binding Contract query summary (L11-AR).
/// Business-agnostic: state + optional generation; never tools/prompt body or business IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BindingStateSummary {
    pub state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<u64>,
}

/// Stable Binding Contract Set failure category (L11-AR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetError {
    code: &'static str,
}

impl SetError {
    pub fn set_invalid() -> Self {
        Self {
            code: "set_invalid",
        }
    }

    pub fn as_code(&self) -> &'static str {
        self.code
    }
}

fn tools_applicable(tools: &Value) -> bool {
    match tools {
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        Value::String(s) => !s.trim().is_empty(),
        _ => false,
    }
}

fn prompt_applicable(prompt: &Value) -> bool {
    match prompt {
        Value::String(s) => !s.trim().is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => false,
    }
}

fn callbacks_slot_ok(callbacks: &Value) -> bool {
    callbacks.is_object()
}

/// B1 Set validation: three slots required; tools/prompt must be applicable.
pub fn validate_binding(binding: &Binding) -> Result<(), SetError> {
    if !tools_applicable(&binding.tools)
        || !prompt_applicable(&binding.prompt)
        || !callbacks_slot_ok(&binding.callbacks)
    {
        return Err(SetError::set_invalid());
    }
    Ok(())
}

/// Parse Binding from JSON requiring tools/prompt/callbacks keys; never fill from other fields.
pub fn binding_from_json(v: &Value) -> Result<Binding, SetError> {
    let obj = v.as_object().ok_or_else(SetError::set_invalid)?;
    if !obj.contains_key("tools") || !obj.contains_key("prompt") || !obj.contains_key("callbacks")
    {
        return Err(SetError::set_invalid());
    }
    Ok(Binding {
        tools: obj["tools"].clone(),
        prompt: obj["prompt"].clone(),
        callbacks: obj["callbacks"].clone(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_master_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_title: Option<String>,
    #[serde(default)]
    pub turns: Vec<Turn>,
}

pub fn agent_dir() -> Result<PathBuf, String> {
    Ok(paths::cache_dir()
        .map_err(|e| format!("{e:?}"))?
        .join("agent"))
}

pub fn agent_log_dir() -> Result<PathBuf, String> {
    agent_dir()
}

pub fn sessions_dir() -> Result<PathBuf, String> {
    Ok(agent_dir()?.join("sessions"))
}

pub fn session_file_path(session_id: &str) -> Result<PathBuf, String> {
    let id = session_id.trim();
    if id.is_empty() {
        return Err("Missing session_id".into());
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err("Invalid session_id".into());
    }
    Ok(sessions_dir()?.join(format!("{id}.json")))
}

pub fn create_session(
    bound_master_task_id: Option<&str>,
    bound_title: Option<&str>,
) -> Result<Session, String> {
    let session = Session {
        session_id: format!("sess_{}", random_hex12()),
        bound_master_task_id: bound_master_task_id
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        bound_title: bound_title
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        turns: Vec::new(),
    };
    save_session(&session)?;
    Ok(session)
}

pub fn save_session(session: &Session) -> Result<(), String> {
    let path = session_file_path(&session.session_id)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_session(session_id: &str) -> Result<Session, String> {
    let path = session_file_path(session_id)?;
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Read-only turns for binding hydrate. Empty when no live session / load fails.
pub fn load_turns_value(session_id: &str) -> Value {
    let id = session_id.trim();
    if id.is_empty() {
        return Value::Array(Vec::new());
    }
    match load_session(id) {
        Ok(session) => serde_json::to_value(&session.turns).unwrap_or_else(|_| Value::Array(Vec::new())),
        Err(_) => Value::Array(Vec::new()),
    }
}

pub fn append_turn(session_id: &str, turn: Turn) -> Result<Session, String> {
    let mut session = load_session(session_id)?;
    session.turns.push(turn);
    save_session(&session)?;
    Ok(session)
}

/// Redact secrets before writing agent error logs.
pub fn redact_secrets(message: &str) -> String {
    let mut out = message.to_string();
    // Authorization: Bearer …
    if let Some(idx) = out.to_ascii_lowercase().find("authorization:") {
        let rest = &out[idx..];
        let end = rest.find(['\n', '\r']).unwrap_or(rest.len());
        let replacement = "Authorization: [REDACTED]";
        out.replace_range(idx..idx + end, replacement);
    }
    // Bearer tokens elsewhere
    while let Some(idx) = out.to_ascii_lowercase().find("bearer ") {
        let start = idx;
        let rest = &out[start + "bearer ".len()..];
        let end_rel = rest
            .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
            .unwrap_or(rest.len());
        let end = start + "bearer ".len() + end_rel;
        out.replace_range(start..end, "Bearer [REDACTED]");
    }
    // api_key=… / "api_key":"…"
    let patterns = ["api_key=", "\"api_key\":\""];
    for pat in patterns {
        let lower = out.to_ascii_lowercase();
        if let Some(idx) = lower.find(pat) {
            let value_start = idx + pat.len();
            let rest = &out[value_start..];
            let end_rel = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ',' || c == '}')
                .unwrap_or(rest.len());
            out.replace_range(value_start..value_start + end_rel, "[REDACTED]");
        }
    }
    out
}

pub fn log_agent_error(message: &str) -> Result<(), String> {
    let dir = agent_log_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("agent-error.log");
    let line = format!(
        "{} {}\n",
        Utc::now().to_rfc3339(),
        redact_secrets(message)
    );
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    f.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
    Ok(())
}
