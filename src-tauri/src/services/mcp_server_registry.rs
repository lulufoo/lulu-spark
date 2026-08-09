//! Host-authoritative business key → MCP Server config registry.
//!
//! Value is a decision-level connection/capability description plus a structured
//! HTTP transport definition consumable by Cursor Agent SDK Local (`mcpServers`).
//! Endpoint *readiness* is owned by [`crate::services::mcp_endpoint_readiness`] —
//! a candidate localhost URL here is never treated as ready by itself.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

/// Seeded business key for Binding assembly (todo_task surface).
pub const SEEDED_BUSINESS_KEY: &str = "todo_task";

/// Default inline mcpServers entry name for the Workbench Host MCP surface.
pub const DEFAULT_HTTP_MCP_SERVER_NAME: &str = "workbench";

/// Structured HTTP MCP transport for SDK consumption (name / URL / headers).
/// Presence of this value is not readiness — probe via `mcp_endpoint_readiness`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpMcpTransport {
    pub name: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
}

/// Decision-level MCP Server connection/capability description + HTTP transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerConfig {
    pub capability_description: String,
    pub http_transport: HttpMcpTransport,
}

impl McpServerConfig {
    /// Expose the SDK-consumable HTTP MCP transport definition.
    pub fn http_transport(&self) -> &HttpMcpTransport {
        &self.http_transport
    }
}

/// Explicit lookup/register failure — never silently return empty config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpServerLookupError {
    NotFound,
    InvalidKey,
}

/// Decision-level description for the L1 internal MCP `todo_task` surface.
const SEEDED_TODO_CAPABILITY_DESCRIPTION: &str =
    "internal host-mcp todo_task capability surface";

fn seeded_http_transport() -> HttpMcpTransport {
    let mut headers = BTreeMap::new();
    headers.insert(
        "Accept".to_string(),
        "application/json, text/event-stream".to_string(),
    );
    HttpMcpTransport {
        name: DEFAULT_HTTP_MCP_SERVER_NAME.to_string(),
        url: format!(
            "http://127.0.0.1:{}/mcp/{}",
            crate::config::settings::load()
                .map(|s| s.effective_mcp_port())
                .unwrap_or(crate::DEFAULT_MCP_PORT),
            SEEDED_BUSINESS_KEY
        ),
        headers,
    }
}

fn seeded_config() -> McpServerConfig {
    McpServerConfig {
        capability_description: SEEDED_TODO_CAPABILITY_DESCRIPTION.to_string(),
        http_transport: seeded_http_transport(),
    }
}

fn table() -> &'static Mutex<HashMap<String, McpServerConfig>> {
    static TABLE: OnceLock<Mutex<HashMap<String, McpServerConfig>>> = OnceLock::new();
    // Host-authoritative defaults must exist on first access (production Set path),
    // not only when tests call `seed_defaults`.
    TABLE.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(SEEDED_BUSINESS_KEY.to_string(), seeded_config());
        Mutex::new(map)
    })
}

fn validate_key(key: &str) -> Result<(), McpServerLookupError> {
    if key.trim().is_empty() {
        Err(McpServerLookupError::InvalidKey)
    } else {
        Ok(())
    }
}

fn validate_config(config: &McpServerConfig) -> Result<(), McpServerLookupError> {
    let t = &config.http_transport;
    if config.capability_description.trim().is_empty()
        || t.name.trim().is_empty()
        || t.url.trim().is_empty()
    {
        return Err(McpServerLookupError::InvalidKey);
    }
    Ok(())
}

/// Register a business key → MCP Server config mapping (Host-internal).
pub fn register(key: &str, config: McpServerConfig) -> Result<(), McpServerLookupError> {
    validate_key(key)?;
    validate_config(&config)?;
    let mut guard = table().lock().expect("mcp_server_registry lock");
    guard.insert(key.to_string(), config);
    Ok(())
}

/// Look up MCP Server config by business key.
pub fn lookup(key: &str) -> Result<McpServerConfig, McpServerLookupError> {
    validate_key(key)?;
    let guard = table().lock().expect("mcp_server_registry lock");
    guard
        .get(key)
        .cloned()
        .ok_or(McpServerLookupError::NotFound)
}

/// Seed (or re-seed after test clear) the L1-backed business key for Binding assembly.
pub fn seed_defaults() {
    let _ = register(SEEDED_BUSINESS_KEY, seeded_config());
}

/// Test helper: reset Host-internal registry state.
#[cfg(test)]
pub fn clear_for_tests() {
    let mut guard = table().lock().expect("mcp_server_registry lock");
    guard.clear();
}

#[cfg(test)]
#[path = "../unit-tests/services/mcp_server_registry_tests.rs"]
mod tests;
