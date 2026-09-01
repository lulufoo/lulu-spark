use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::services::path_fence::PathFence;
use crate::mcp_host::registry::McpServerConfig;

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
    pub(crate) current_binding: Option<Binding>,
    pub(crate) current_business_id: Option<String>,
    pub(crate) loaded_mcp_server: Option<McpServerConfig>,
    pub(crate) loaded_path_fence: Option<PathFence>,
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

    pub fn loaded_path_fence(&self) -> Option<PathFence> {
        self.loaded_path_fence.clone()
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

/// Chat-scoped Stage registration. Path metadata only — never file body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagedEntry {
    pub id: String,
    pub path: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub session_id: String,
    #[serde(default)]
    pub turns: Vec<Turn>,
    #[serde(default)]
    pub staged: Vec<StagedEntry>,
}

impl Session {
    /// Register a path on this Chat's Stage (metadata only; no file body).
    pub fn register_staged(
        &mut self,
        path: &str,
        title: Option<&str>,
    ) -> Result<StagedEntry, String> {
        let path = path.trim();
        if path.is_empty() {
            return Err("missing path".to_string());
        }
        let title = title
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| default_staged_title(path));
        let entry = StagedEntry {
            id: next_staged_handle(&self.staged),
            path: path.to_string(),
            title,
        };
        self.staged.push(entry.clone());
        Ok(entry)
    }

    pub fn list_staged(&self) -> &[StagedEntry] {
        &self.staged
    }

    pub fn get_staged(&self, id: &str) -> Option<&StagedEntry> {
        self.staged.iter().find(|entry| entry.id == id)
    }
}

fn default_staged_title(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn parse_staged_handle(id: &str) -> Option<u32> {
    let rest = id.strip_prefix('F')?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    rest.parse().ok()
}

fn next_staged_handle(staged: &[StagedEntry]) -> String {
    let high = staged
        .iter()
        .filter_map(|e| parse_staged_handle(&e.id))
        .max()
        .unwrap_or(0);
    format!("F{}", high + 1)
}
