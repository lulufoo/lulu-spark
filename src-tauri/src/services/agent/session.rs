//! Session persistence under `{cache_dir}/agent/sessions/`.
//!
//! `AIAssistantSession` is the sole live context owner (binding / MCP / live session
//! id / cancel). Disk `Session` cache remains the only turns persistence backend.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::services::id::random_hex12;
use crate::services::mcp_server_registry::McpServerConfig;

/// P1 / T1: session lifecycle entry (`create_session` / persist) does not expose
/// an engine-selection API — no Host/Cursor parameter on create or session record.
pub const SESSION_LIFECYCLE_ENGINE_OPAQUE: bool = true;

const ENGINE_SELECTION_KEYS: &[&str] = &[
    "engine",
    "engine_type",
    "engineType",
    "assistant_engine",
];

/// True when a JSON value (recursively) carries engine-selection fields that
/// would let a caller branch on Host vs Cursor.
pub fn value_exposes_engine_selection(v: &Value) -> bool {
    match v {
        Value::Object(map) => {
            if ENGINE_SELECTION_KEYS.iter().any(|k| map.contains_key(*k)) {
                return true;
            }
            map.values().any(value_exposes_engine_selection)
        }
        Value::Array(items) => items.iter().any(value_exposes_engine_selection),
        _ => false,
    }
}

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

/// Binding Contract surface.
///
/// Public Set input is key-only (`binding_from_json`). Host may still hold
/// tools/prompt/callbacks as an internal execute/chat surface (typed `set_binding`
/// / tests) until L3 empties tools; those slots are rejected at the public JSON boundary.
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

    pub fn unknown_key() -> Self {
        Self {
            code: "unknown_key",
        }
    }

    pub fn as_code(&self) -> &'static str {
        self.code
    }
}

fn tools_applicable(tools: &Value) -> bool {
    match tools {
        // Empty array is legal: Host Agent business session may submit tools:[].
        Value::Array(_) => true,
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

const KEY_BINDING_TOOL_NAME: &str = "__binding_key__";

/// Business key carried by a key-only Binding (public Set contract).
pub fn binding_business_key(binding: &Binding) -> Option<String> {
    let arr = binding.tools.as_array()?;
    let first = arr.first()?;
    if first.get("name").and_then(|n| n.as_str()) != Some(KEY_BINDING_TOOL_NAME) {
        return None;
    }
    let key = first.get("handle").and_then(|h| h.as_str())?.trim();
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

/// Set validation: tools/prompt must be applicable; callbacks slot present.
pub fn validate_binding(binding: &Binding) -> Result<(), SetError> {
    if !tools_applicable(&binding.tools)
        || !prompt_applicable(&binding.prompt)
        || !callbacks_slot_ok(&binding.callbacks)
    {
        return Err(SetError::set_invalid());
    }
    Ok(())
}

/// Parse key-only Binding input. Rejects legacy `tools`/`prompt`/`callbacks` payload
/// and engine selection parameters. Side-effect free (no registry I/O).
pub fn binding_from_json(v: &Value) -> Result<Binding, SetError> {
    let obj = v.as_object().ok_or_else(SetError::set_invalid)?;
    // Legacy payload must not bypass key→MCP lookup at the public boundary.
    if obj.contains_key("tools") || obj.contains_key("prompt") || obj.contains_key("callbacks") {
        return Err(SetError::set_invalid());
    }
    if value_exposes_engine_selection(v) {
        return Err(SetError::set_invalid());
    }
    let key = match obj.get("key") {
        Some(Value::String(s)) => s.clone(),
        _ => return Err(SetError::set_invalid()),
    };
    if key.trim().is_empty() {
        return Err(SetError::set_invalid());
    }
    // Host-internal placeholders only — not client-supplied MCP/tools payload.
    Ok(Binding {
        tools: json!([{ "name": KEY_BINDING_TOOL_NAME, "handle": key }]),
        prompt: json!("pending"),
        callbacks: json!({}),
    })
}

/// Detached execution-context snapshot for engine adapters (L2-A).
/// Consumed read-only; engines must not own or persist business session state.
#[derive(Debug, Clone, PartialEq)]
pub struct LiveExecContext {
    pub session_id: Option<String>,
    pub binding: Option<Binding>,
    pub loaded_mcp_server: Option<McpServerConfig>,
    pub generation: Option<u64>,
}

/// Business-held logical dialogue instance and sole live context owner.
///
/// Allocates / holds live `session_id`, business binding, MCP capability, generation,
/// and cancel flags. Persists turns only through the existing Session cache — never
/// a parallel turn store. Engine/model config is not session state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AIAssistantSession {
    pub(crate) current_session_id: Option<String>,
    pub(crate) bound_master_task_id: Option<String>,
    pub(crate) bound_title: Option<String>,
    pub(crate) current_binding: Option<Binding>,
    pub(crate) loaded_mcp_server: Option<McpServerConfig>,
    pub(crate) current_generation: Option<u64>,
    pub(crate) generation_seq: u64,
    pub(crate) execute_cancelled: bool,
    pub(crate) chat_cancelled: bool,
}

impl AIAssistantSession {
    /// Contract marker: live owner must not duplicate Session turns storage.
    pub const HOLDS_PARALLEL_TURN_STORAGE: bool = false;

    pub fn current_session_id(&self) -> Option<String> {
        self.current_session_id.clone()
    }

    pub fn current_binding(&self) -> Option<Binding> {
        self.current_binding.clone()
    }

    pub fn loaded_mcp_server(&self) -> Option<McpServerConfig> {
        self.loaded_mcp_server.clone()
    }

    pub fn current_generation(&self) -> Option<u64> {
        self.current_generation
    }

    pub fn chat_cancelled(&self) -> bool {
        self.chat_cancelled
    }

    pub fn execute_cancelled(&self) -> bool {
        self.execute_cancelled
    }

    pub fn execution_context_snapshot(&self) -> LiveExecContext {
        LiveExecContext {
            session_id: self.current_session_id.clone(),
            binding: self.current_binding.clone(),
            loaded_mcp_server: self.loaded_mcp_server.clone(),
            generation: self.current_generation,
        }
    }
}

fn live_slot() -> &'static Mutex<AIAssistantSession> {
    static LIVE: OnceLock<Mutex<AIAssistantSession>> = OnceLock::new();
    LIVE.get_or_init(|| Mutex::new(AIAssistantSession::default()))
}

/// Clone of the sole live context owner (binding / MCP / session_id / cancel).
pub fn live_context_owner() -> AIAssistantSession {
    live_slot()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

pub(crate) fn with_live_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut AIAssistantSession) -> R,
{
    let mut guard = live_slot().lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

pub fn reset_live_for_tests() {
    *live_slot().lock().unwrap_or_else(|e| e.into_inner()) = AIAssistantSession::default();
}

/// ProcessCursorRunnerClient / Node runner failure must never switch business session.
/// Test hook: invoke after a simulated client failure; live owner stays unchanged.
pub fn note_runner_client_failure_for_tests() {
    // Intentionally no-op on live context — failure is engine/process scoped only.
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
        Ok(session) => serde_json::to_value(&session.turns)
            .unwrap_or_else(|_| Value::Array(Vec::new())),
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
