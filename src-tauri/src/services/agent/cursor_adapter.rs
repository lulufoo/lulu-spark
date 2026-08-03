//! Cursor Local adapter — Agent SDK Local shape + inline mcpServers.
//!
//! Isolated from Host Loop (`loop.rs` / `llm.rs` must not import this module).
//! Cursor failures stay on the SDK path (L09-I #7 — no fallback to
//! process-local business tool dispatch).
//!
//! ## A1 (L3 Must Close Before P4 / F-46–F-47) — cwd + mcpServers combo
//!
//! Confirmed: Agent SDK Local `Agent.create` receives per-session
//! independent [`local.cwd`](AgentCreateParams::local_cwd) together with
//! inline [`mcpServers`](AgentCreateParams::mcp_servers). Lifecycle for cwd
//! is owned by [`crate::services::agent::session_cwd`]; use
//! [`run_turn_for_session`] to allocate then inject.
//!
//! ## A2 (L3 Must Close Before F-46) — SDK embed strategy
//!
//! Real Cursor Agent SDK crate is not available in this workspace. Embedding is
//! narrowed to a replaceable [`CursorAgentSdk`] port/trait:
//! - Production can later supply a real SDK implementation behind the same port.
//! - Tests use a capturing double that records `Agent.create` params
//!   (`mcpServers` + `local.cwd`).
//! - Host engine path is not forced to link any Cursor SDK (module isolation).

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use crate::services::agent::engine_router::TurnInput;
use crate::services::agent::r#loop;
use crate::services::agent::session_cwd;
use crate::services::mcp_server_registry::McpServerConfig;

/// A1 confirmed: per-session `local.cwd` combined with injected `mcpServers`
/// on the same Agent.create params (see `run_turn_for_session`).
pub const CURSOR_LOCAL_A1_CWD_MCPSERVERS_COMBO: bool = true;

/// A2 narrowed (not silently assumed): SDK embed via replaceable port.
/// Documented in module docs + code_log delivery note.
pub const CURSOR_SDK_A2_REPLACEABLE_PORT: bool = true;

/// Default inline mcpServers entry name for the session capability config.
pub const DEFAULT_MCP_SERVER_NAME: &str = "workbench";

/// Decision-level entry inside Agent SDK Local inline `mcpServers`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpServerInlineEntry {
    pub capability_description: String,
}

/// Agent SDK Local inline `mcpServers` object (name → config).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct McpServersInline {
    #[serde(flatten)]
    pub servers: BTreeMap<String, McpServerInlineEntry>,
}

/// Captured / passed `Agent.create` parameters (Local shape).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCreateParams {
    pub mcp_servers: McpServersInline,
    /// Injection point for Agent SDK Local `local.cwd` (lifecycle owned by t5).
    pub local_cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorAdapterError {
    MissingMcpConfig,
    /// Per-session cwd allocation failed — turn must not start; no shared cwd.
    Cwd(String),
    Sdk(String),
}

impl std::fmt::Display for CursorAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CursorAdapterError::MissingMcpConfig => {
                write!(f, "session capability MCP config is not loaded")
            }
            CursorAdapterError::Cwd(msg) => write!(f, "session cwd error: {msg}"),
            CursorAdapterError::Sdk(msg) => write!(f, "cursor sdk error: {msg}"),
        }
    }
}

impl std::error::Error for CursorAdapterError {}

/// Replaceable port for Cursor Agent SDK Local (`Agent.create` + turn execution).
///
/// Host path must not depend on a concrete Cursor SDK crate; inject via this trait.
pub trait CursorAgentSdk {
    fn create(&mut self, params: AgentCreateParams) -> Result<(), CursorAdapterError>;
    fn run_turn(&mut self, message: &str) -> Result<String, CursorAdapterError>;
}

/// Map L2 decision-level [`McpServerConfig`] → Agent SDK Local inline `mcpServers`.
pub fn map_mcp_config_to_mcp_servers(config: &McpServerConfig) -> McpServersInline {
    let mut servers = BTreeMap::new();
    servers.insert(
        DEFAULT_MCP_SERVER_NAME.to_string(),
        McpServerInlineEntry {
            capability_description: config.capability_description.clone(),
        },
    );
    McpServersInline { servers }
}

/// `Agent.create` with inline `mcpServers` + `local.cwd` placeholder.
pub fn create_agent<S: CursorAgentSdk>(
    sdk: &mut S,
    mcp_config: &McpServerConfig,
    cwd: PathBuf,
) -> Result<(), CursorAdapterError> {
    let params = AgentCreateParams {
        mcp_servers: map_mcp_config_to_mcp_servers(mcp_config),
        local_cwd: cwd,
    };
    sdk.create(params)
}

/// Run a Cursor Local turn: read L2 `session_capability_mcp_config()`, create
/// agent with inline mcpServers + `local.cwd`, then execute.
/// Never falls back to process-local business tools on failure.
pub fn run_turn<S: CursorAgentSdk>(
    sdk: &mut S,
    input: &TurnInput,
    cwd: PathBuf,
) -> Result<String, CursorAdapterError> {
    let Some(mcp) = r#loop::session_capability_mcp_config() else {
        return Err(CursorAdapterError::MissingMcpConfig);
    };
    create_agent(sdk, &mcp, cwd)?;
    sdk.run_turn(&input.message)
}

/// Allocate per-session cwd, then run a Cursor turn with A1 combo
/// (`local.cwd` + `mcpServers`). Create failure → explicit error; does not
/// start the SDK turn and never falls back to shared cwd or process-local
/// business tool dispatch.
pub fn run_turn_for_session<S: CursorAgentSdk>(
    sdk: &mut S,
    input: &TurnInput,
) -> Result<String, CursorAdapterError> {
    let cwd = session_cwd::create_session_cwd(&input.session_id).map_err(|e| {
        CursorAdapterError::Cwd(e.to_string())
    })?;
    run_turn(sdk, input, cwd)
}
