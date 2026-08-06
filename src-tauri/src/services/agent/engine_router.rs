//! Settings → adapter routing (P2 / T2) + runtime config read (P4 / T4).
//!
//! Engine selection is read from `AppSettings.assistant_engine` only.
//! Public session facade APIs must not accept engine parameters.
//! `read_engine_runtime_config` aggregates category + model + credential for
//! adapters; it is read-only and does not expose SDK/cwd/MCP as settings fields.

use serde::Serialize;

use crate::config::secrets::{self, KEY_LLM_API_KEY, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings::{self, AppSettings};

/// Selected assistant engine (Host Loop vs Cursor Local).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Host,
    Cursor,
}

/// Read-only snapshot for router/adapter consumption (no cwd / SDK / MCP fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineRuntimeConfig {
    pub engine: EngineKind,
    pub model: String,
    pub credential: Option<String>,
}

/// Which adapter stub/path was entered (internal observability; not a facade field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    Host,
    Cursor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineRouteError {
    InvalidEngine(String),
    Adapter(String),
}

impl std::fmt::Display for EngineRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineRouteError::InvalidEngine(v) => {
                write!(f, "invalid assistant_engine value: {v}")
            }
            EngineRouteError::Adapter(msg) => write!(f, "adapter error: {msg}"),
        }
    }
}

impl std::error::Error for EngineRouteError {}

/// Minimal turn inputs for dispatch (adapters own richer shapes in t3/t4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnInput {
    pub session_id: String,
    pub message: String,
}

/// Internal route outcome — must not expose engine-selection keys to callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteOutcome {
    pub adapter: AdapterKind,
    pub body: String,
}

/// Read engine selection from settings.
///
/// - missing / empty / whitespace → `Host` (no panic)
/// - `"host"` / `"cursor"` (case-insensitive) → corresponding kind
/// - any other value → `Err` (no silent fallback)
pub fn resolve_engine(settings: &AppSettings) -> Result<EngineKind, EngineRouteError> {
    let raw = settings.assistant_engine.trim();
    if raw.is_empty() {
        return Ok(EngineKind::Host);
    }
    match raw.to_ascii_lowercase().as_str() {
        "host" => Ok(EngineKind::Host),
        "cursor" => Ok(EngineKind::Cursor),
        _ => Err(EngineRouteError::InvalidEngine(raw.to_string())),
    }
}

/// Aggregate current engine category + model + associated credential (secrets).
///
/// Read-only: does not write settings/secrets and does not surface SDK/cwd/MCP.
/// Illegal category → same `Err` as `resolve_engine`. Missing credential → `None`.
pub fn read_engine_runtime_config(
    settings: &AppSettings,
) -> Result<EngineRuntimeConfig, EngineRouteError> {
    let engine = resolve_engine(settings)?;
    // Look up by resolved kind so blank/whitespace assistant_engine (→ Host)
    // still reads the active Host list entry, not an empty miss.
    let type_key = match engine {
        EngineKind::Host => "host",
        EngineKind::Cursor => "cursor",
    };
    let model = settings::llm_entry_by_type(&settings.llm, type_key)
        .map(|e| e.model.clone())
        .unwrap_or_default();
    let secret_key = match engine {
        EngineKind::Host => KEY_LLM_API_KEY,
        EngineKind::Cursor => KEY_LLM_API_KEY_CURSOR,
    };
    let credential = secrets::get_secret(secret_key)
        .ok()
        .flatten()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    Ok(EngineRuntimeConfig {
        engine,
        model,
        credential,
    })
}

/// Alias for adapter/route call sites that prefer settings-oriented naming.
pub fn engine_settings_for_route(
    settings: &AppSettings,
) -> Result<EngineRuntimeConfig, EngineRouteError> {
    read_engine_runtime_config(settings)
}

/// Dispatch a chat turn to the Host or Cursor adapter entry.
///
/// Closures are injectable so callers/tests supply adapter bodies — Cursor path
/// should invoke `cursor_adapter::run_turn` (mcpServers + local.cwd). Engine kind
/// is not written into facade API params.
pub fn route_chat_turn<H, C>(
    engine: EngineKind,
    _input: &TurnInput,
    host_adapter: H,
    cursor_adapter: C,
) -> Result<RouteOutcome, EngineRouteError>
where
    H: FnOnce() -> Result<String, String>,
    C: FnOnce() -> Result<String, String>,
{
    match engine {
        EngineKind::Host => {
            let body = host_adapter().map_err(EngineRouteError::Adapter)?;
            Ok(RouteOutcome {
                adapter: AdapterKind::Host,
                body,
            })
        }
        EngineKind::Cursor => {
            let body = cursor_adapter().map_err(EngineRouteError::Adapter)?;
            Ok(RouteOutcome {
                adapter: AdapterKind::Cursor,
                body,
            })
        }
    }
}

/// Convenience: resolve from settings then dispatch (settings-only selection).
pub fn dispatch_from_settings<H, C>(
    settings: &AppSettings,
    input: &TurnInput,
    host_adapter: H,
    cursor_adapter: C,
) -> Result<RouteOutcome, EngineRouteError>
where
    H: FnOnce() -> Result<String, String>,
    C: FnOnce() -> Result<String, String>,
{
    let engine = resolve_engine(settings)?;
    route_chat_turn(engine, input, host_adapter, cursor_adapter)
}
