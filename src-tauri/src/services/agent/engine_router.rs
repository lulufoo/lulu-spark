//! Settings → adapter routing (P2 / T2).
//!
//! Engine selection is read from `AppSettings.assistant_engine` only.
//! Public session facade APIs must not accept engine parameters.

use serde::Serialize;

use crate::config::settings::AppSettings;

/// Selected assistant engine (Host Loop vs Cursor Local).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Host,
    Cursor,
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

/// Dispatch a chat turn to the Host or Cursor adapter entry (stubs OK for t2).
///
/// Closures are injectable so tests can observe which path ran; t3/t4 replace
/// bodies with real adapters. Engine kind is not written into facade API params.
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
