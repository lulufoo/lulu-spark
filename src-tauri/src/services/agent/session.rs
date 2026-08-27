//! Session persistence under `{cache_dir}/agent/sessions/`.
//!
//! `AIAssistantSession` is the sole live context owner (binding / MCP / live session
//! id / cancel). Disk `Session` cache remains the only turns persistence backend.

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use chrono::{Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::services::id::random_hex12;
use crate::services::mcp_server_registry::McpServerConfig;

/// Session lifecycle entry (`create_session` / persist) does not expose an
/// assistant-channel selection API or store it on the session record.
pub const SESSION_LIFECYCLE_ENGINE_OPAQUE: bool = true;

const ENGINE_SELECTION_KEYS: &[&str] = &[
    "engine",
    "engine_type",
    "engineType",
    "assistant_engine",
];

/// True when a JSON value (recursively) carries assistant-channel selection
/// fields that would let a caller branch on the configured channel.
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

/// Business routing identity derived from the current key-only Binding.
///
/// Public key-only parsing keeps the key in a Host-owned live slot rather than
/// exposing it as executable tools. The live chat `session_id` remains
/// history/cancellation metadata and is deliberately not an alternative source
/// for Agent selection.
pub fn binding_business_id(binding: &Binding) -> Option<String> {
    if let Some(key) = binding_business_key(binding) {
        return Some(key);
    }
    let live = live_context_owner();
    (live.current_binding.as_ref() == Some(binding))
        .then_some(live.current_business_id)
        .flatten()
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
    pub business_id: Option<String>,
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
    pub(crate) current_business_id: Option<String>,
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

    pub fn current_business_id(&self) -> Option<String> {
        self.current_business_id.clone()
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
            business_id: self.current_business_id.clone(),
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

/// Conversation files live under `{cache_dir}/agent/sessions`.
/// Tests must use `TestSandbox` so they cannot read or write the user's store.
pub fn sessions_dir() -> Result<PathBuf, String> {
    let dir = agent_dir()?.join("sessions");
    #[cfg(test)]
    {
        reject_unisolated_sessions_dir(&dir)?;
    }
    Ok(dir)
}

#[cfg(test)]
fn reject_unisolated_sessions_dir(dir: &PathBuf) -> Result<(), String> {
    use crate::config::settings;
    if !settings::is_test_sandbox() {
        return Err("assistant conversation directory requires TestSandbox isolation".into());
    }
    let prod_cache = settings::load_prod_settings().cache_dir;
    if dir == &prod_cache || dir.starts_with(&prod_cache) {
        return Err(format!(
            "assistant conversation directory must not use prod cache {}",
            prod_cache.display()
        ));
    }
    Ok(())
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

const HOME_CHAT_LIST_LIMIT: usize = 20;

fn format_session_when(updated_at: i64) -> String {
    if updated_at <= 0 {
        return "New conversation".to_string();
    }
    Local
        .timestamp_opt(updated_at, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "New conversation".to_string())
}

fn session_list_title(session: &Session, updated_at: i64) -> String {
    if let Some(title) = session
        .bound_title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return title.to_string();
    }
    session
        .turns
        .iter()
        .find(|turn| turn.role == "user")
        .and_then(|turn| turn.content.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|text| {
            let clipped: String = text.chars().take(48).collect();
            if text.chars().count() > 48 {
                format!("{clipped}…")
            } else {
                clipped
            }
        })
        .unwrap_or_else(|| format_session_when(updated_at))
}

fn file_updated_at(path: &PathBuf) -> i64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Disk session catalog for the Home history list. Newest first.
pub fn list_session_summaries() -> Result<Vec<Value>, String> {
    let dir = sessions_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut items: Vec<(i64, Value)> = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem.ends_with(".json") || stem.contains('.') {
            continue;
        }
        let Ok(session) = load_session(stem) else {
            continue;
        };
        let updated_at = file_updated_at(&path);
        items.push((
            updated_at,
            json!({
                "session_id": session.session_id,
                "title": session_list_title(&session, updated_at),
                "updated_at": updated_at,
            }),
        ));
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    items.truncate(HOME_CHAT_LIST_LIMIT);
    Ok(items.into_iter().map(|(_, v)| v).collect())
}

/// Read-only turns for Home / binding hydrate. Empty when no live session / load fails.
///
/// User and assistant text only. Tool dumps stay on disk and must not ride the
/// IPC — a single `list_todo_tasks` result can be hundreds of KB.
pub fn load_turns_value(session_id: &str) -> Value {
    let id = session_id.trim();
    if id.is_empty() {
        return Value::Array(Vec::new());
    }
    match load_session(id) {
        Ok(session) => Value::Array(
            session
                .turns
                .iter()
                .filter(|turn| {
                    (turn.role == "user" || turn.role == "assistant")
                        && turn
                            .content
                            .as_deref()
                            .map(|text| !text.is_empty())
                            .unwrap_or(false)
                })
                .map(|turn| {
                    json!({
                        "role": turn.role,
                        "content": turn.content,
                    })
                })
                .collect(),
        ),
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
