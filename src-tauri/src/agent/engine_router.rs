//! Settings → Agent Loop routing and runtime configuration.
//!
//! The Host/GLM path is the only supported assistant channel. Legacy or
//! unknown `assistant_engine` values remain untouched in local settings but
//! resolve to an unconfigured state and never invoke an Agent.

use serde::Serialize;

use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings::{self, AppSettings};

/// The only supported assistant engine: Agent Loop backed by GLM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    Host,
}

/// Read-only snapshot consumed by the Host LLM path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineRuntimeConfig {
    pub engine: EngineKind,
    pub model: String,
    pub credential: Option<String>,
}

/// The only adapter path exposed by the router.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    Host,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineRouteError {
    /// Empty, legacy, or unknown settings do not select an assistant.
    Unconfigured,
    Adapter(String),
}

impl std::fmt::Display for EngineRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineRouteError::Unconfigured => {
                write!(f, "assistant engine is not configured; configure Host / GLM")
            }
            EngineRouteError::Adapter(message) => write!(f, "adapter error: {message}"),
        }
    }
}

impl std::error::Error for EngineRouteError {}

/// Minimal turn inputs for dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnInput {
    pub session_id: String,
    pub message: String,
}

/// Internal route outcome — engine selection stays out of the session facade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteOutcome {
    pub adapter: AdapterKind,
    pub body: String,
}

/// Resolve the only supported engine from settings.
///
/// `host` selects Agent Loop. Empty, legacy, and every other value are
/// deliberately treated as unconfigured instead of silently falling back.
pub fn resolve_engine(settings: &AppSettings) -> Result<EngineKind, EngineRouteError> {
    if settings.assistant_engine.trim().eq_ignore_ascii_case("host") {
        Ok(EngineKind::Host)
    } else {
        Err(EngineRouteError::Unconfigured)
    }
}

/// Aggregate the Host/GLM model and credential without changing settings.
pub fn read_engine_runtime_config(
    settings: &AppSettings,
) -> Result<EngineRuntimeConfig, EngineRouteError> {
    let engine = resolve_engine(settings)?;
    let model = settings::llm_entry_by_type(&settings.llm, "host")
        .map(|entry| entry.model.clone())
        .unwrap_or_default();
    let credential = secrets::get_secret(KEY_LLM_API_KEY)
        .ok()
        .flatten()
        .map(|secret| secret.trim().to_string())
        .filter(|secret| !secret.is_empty());

    Ok(EngineRuntimeConfig {
        engine,
        model,
        credential,
    })
}

/// Dispatch a turn through the single Host adapter.
pub fn route_chat_turn<H>(
    engine: EngineKind,
    _input: &TurnInput,
    host_adapter: H,
) -> Result<RouteOutcome, EngineRouteError>
where
    H: FnOnce() -> Result<String, String>,
{
    match engine {
        EngineKind::Host => {
            let body = host_adapter().map_err(EngineRouteError::Adapter)?;
            Ok(RouteOutcome {
                adapter: AdapterKind::Host,
                body,
            })
        }
    }
}

/// Resolve settings and dispatch through Agent Loop.
pub fn dispatch_from_settings<H>(
    settings: &AppSettings,
    input: &TurnInput,
    host_adapter: H,
) -> Result<RouteOutcome, EngineRouteError>
where
    H: FnOnce() -> Result<String, String>,
{
    let engine = resolve_engine(settings)?;
    route_chat_turn(engine, input, host_adapter)
}
