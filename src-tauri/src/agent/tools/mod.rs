//! Turn-facing tool catalog merge and dispatch (Host + Stage + MCP).

pub mod catalog;
pub mod fs;
mod gate;
pub mod host;
mod local;
pub mod stage;
pub mod timeout;

use std::collections::BTreeSet;
use std::fmt;

use serde_json::Value;

use crate::mcp_host::registry::McpServerConfig;
use crate::agent::fence::PathFence;

use super::diagnostics::TurnId;
use super::mcp;
use super::session::Session;

pub use catalog::{ToolCatalog, ToolResult};
pub use local::Stopped;
use timeout::LOCAL_TOOL_TIMEOUT;

/// Prepared tool surface for one agent turn.
pub struct TurnTools {
    pub catalog: Option<ToolCatalog>,
    mcp: Option<(McpServerConfig, ToolCatalog)>,
    /// True when Host file tools + Stage tools were merged (path-fence binding).
    pub host: bool,
}

#[derive(Debug)]
pub enum PrepareError {
    Mcp(mcp::McpError),
    EmptyMcpTools,
}

impl fmt::Display for PrepareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mcp(error) => write!(f, "{error}"),
            Self::EmptyMcpTools => write!(f, "MCP server exposed no tools"),
        }
    }
}

impl std::error::Error for PrepareError {}

fn local_catalog() -> ToolCatalog {
    host::catalog().merge(stage::catalog())
}

/// Discover MCP tools (when configured) and merge with Host+Stage when `with_host`.
pub fn discover_and_merge(
    mcp_config: Option<&McpServerConfig>,
    with_host: bool,
    turn_id: &TurnId,
    session_id: &str,
) -> Result<TurnTools, PrepareError> {
    let mcp = match mcp_config {
        Some(config) => {
            let turn = mcp::McpTurn::new(turn_id, session_id);
            let catalog = mcp::discover_tools(config, &turn).map_err(PrepareError::Mcp)?;
            if catalog.definitions.is_empty() {
                return Err(PrepareError::EmptyMcpTools);
            }
            Some((config.clone(), catalog))
        }
        None => None,
    };

    let host_catalog = if with_host {
        Some(local_catalog())
    } else {
        None
    };

    let catalog = match (
        mcp.as_ref().map(|(_, catalog)| catalog.clone()),
        host_catalog,
    ) {
        (Some(mcp_catalog), Some(host_catalog)) => Some(mcp_catalog.merge(host_catalog)),
        (Some(mcp_catalog), None) => Some(mcp_catalog),
        (None, Some(host_catalog)) => Some(host_catalog),
        (None, None) => None,
    };

    Ok(TurnTools {
        catalog,
        mcp,
        host: with_host,
    })
}

pub fn mcp_names(turn: &TurnTools) -> BTreeSet<String> {
    turn.mcp
        .as_ref()
        .map(|(_, catalog)| catalog.tool_names())
        .unwrap_or_default()
}

/// Local (non-MCP) Host file or Stage tool that should not cross the MCP boundary.
pub fn uses_host(turn: &TurnTools, name: &str) -> bool {
    is_local_builtin(name)
        && turn
            .mcp
            .as_ref()
            .map(|(_, mcp_catalog)| !mcp_catalog.contains(name))
            .unwrap_or(true)
}

fn is_local_builtin(name: &str) -> bool {
    host::is_builtin(name) || stage::is_builtin(name)
}

pub enum InvokeOutcome {
    /// Tool finished (success or tool-level error).
    Done {
        result: ToolResult,
        host: bool,
        /// A local tool did not finish; `result` holds the text for the model.
        stopped: Option<Stopped>,
    },
    /// The turn was cancelled while a local tool was running.
    Cancelled,
    /// Abort the turn with this assistant reply.
    Abort(String),
}

/// Validate arguments and dispatch to Stage / Host / MCP.
///
/// Local tools run under `LOCAL_TOOL_TIMEOUT`; `is_cancelled` is checked while they run.
pub fn invoke(
    turn: &TurnTools,
    name: &str,
    arguments_json: &str,
    fence: Option<&PathFence>,
    session: &mut Session,
    turn_id: &TurnId,
    is_cancelled: &dyn Fn() -> bool,
) -> InvokeOutcome {
    let host = uses_host(turn, name);
    let Some(catalog) = turn.catalog.as_ref() else {
        return InvokeOutcome::Done {
            result: ToolResult {
                content: format!("Tool '{name}' is not available in the active scene."),
                is_error: true,
            },
            host,
            stopped: None,
        };
    };
    if !catalog.contains(name) {
        return InvokeOutcome::Done {
            result: ToolResult {
                content: format!("Tool '{name}' is not available in the active scene."),
                is_error: true,
            },
            host,
            stopped: None,
        };
    }

    let arguments = match serde_json::from_str::<Value>(arguments_json) {
        Ok(value) => value,
        Err(error) => {
            return InvokeOutcome::Done {
                result: ToolResult {
                    content: format!("Tool arguments must be valid JSON: {error}"),
                    is_error: true,
                },
                host,
                stopped: None,
            };
        }
    };

    if let Err(error) = catalog.validate_arguments(name, &arguments) {
        return InvokeOutcome::Done {
            result: ToolResult {
                content: format!("Tool arguments rejected: {error}"),
                is_error: true,
            },
            host,
            stopped: None,
        };
    }

    let result = if host {
        let read_only = !catalog.is_mutating(name);
        match local::call(
            name,
            &arguments,
            session,
            fence,
            read_only,
            LOCAL_TOOL_TIMEOUT,
            is_cancelled,
        ) {
            local::Local::Done(result) => result,
            local::Local::Stopped(stopped, result) => {
                return InvokeOutcome::Done {
                    result,
                    host,
                    stopped: Some(stopped),
                };
            }
            local::Local::Cancelled => return InvokeOutcome::Cancelled,
        }
    } else {
        let Some((mcp_config, _)) = turn.mcp.as_ref() else {
            return InvokeOutcome::Abort(format!(
                "MCP tool '{name}' has no MCP server for this binding."
            ));
        };
        match mcp::call_tool(mcp_config, name, arguments, turn_id, &session.session_id) {
            Ok(result) => result,
            Err(error) => ToolResult {
                content: format!("MCP tool call failed: {error}"),
                is_error: true,
            },
        }
    };

    InvokeOutcome::Done {
        result,
        host,
        stopped: None,
    }
}
