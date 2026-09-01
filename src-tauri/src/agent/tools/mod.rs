//! Turn-facing tool catalog merge and dispatch (Host + Stage + MCP).

pub mod catalog;
pub mod fs;
pub mod host;
pub mod stage;

use std::fmt;

use serde_json::Value;

use crate::mcp_host::registry::McpServerConfig;
use crate::services::path_fence::PathFence;

use super::diagnostics::TraceId;
use super::mcp;
use super::session::Session;

pub use catalog::{ToolCatalog, ToolResult};

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
) -> Result<TurnTools, PrepareError> {
    let mcp = match mcp_config {
        Some(config) => {
            let catalog = mcp::discover_tools(config).map_err(PrepareError::Mcp)?;
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
        /// True when note-content overlay registered a Stage entry.
        staged_note: bool,
    },
    /// Abort the turn with this assistant reply.
    Abort(String),
}

/// Validate arguments, dispatch to Stage / Host / MCP, then run post-invoke overlays.
pub fn invoke(
    turn: &TurnTools,
    name: &str,
    arguments_json: &str,
    fence: Option<&PathFence>,
    session: &mut Session,
    trace_id: &TraceId,
) -> InvokeOutcome {
    let host = uses_host(turn, name);
    let Some(catalog) = turn.catalog.as_ref() else {
        return InvokeOutcome::Done {
            result: ToolResult {
                content: format!("Tool '{name}' is not available in the active scene."),
                is_error: true,
            },
            host,
            staged_note: false,
        };
    };
    if !catalog.contains(name) {
        return InvokeOutcome::Done {
            result: ToolResult {
                content: format!("Tool '{name}' is not available in the active scene."),
                is_error: true,
            },
            host,
            staged_note: false,
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
                staged_note: false,
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
            staged_note: false,
        };
    }

    let result = if host {
        if stage::is_builtin(name) {
            stage::call(name, &arguments, session)
        } else {
            match fence {
                Some(fence) => host::call(name, &arguments, fence),
                None => ToolResult {
                    content: format!("Host tool '{name}' has no path fence for this binding."),
                    is_error: true,
                },
            }
        }
    } else {
        let Some((mcp_config, _)) = turn.mcp.as_ref() else {
            return InvokeOutcome::Abort(format!(
                "MCP tool '{name}' has no MCP server for this binding."
            ));
        };
        match mcp::call_tool(mcp_config, name, arguments, trace_id, &session.session_id) {
            Ok(result) => result,
            Err(error) => ToolResult {
                content: format!("MCP tool call failed: {error}"),
                is_error: true,
            },
        }
    };

    let (result, staged_note) = stage::overlay_note_content(name, result, session);
    InvokeOutcome::Done {
        result,
        host,
        staged_note,
    }
}
