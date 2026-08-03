//! Host-authoritative business key → MCP Server config registry.
//!
//! Value is a decision-level connection/capability description consumable by
//! both engines. Field-level transport schema (stdio/http/…) is deferred
//! (tech-doc T5); this Host table is the hard-to-revert commitment.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Seeded business key for Binding assembly (todo_task surface).
pub const SEEDED_BUSINESS_KEY: &str = "todo_task";

/// Decision-level MCP Server connection/capability description.
/// Field-level schema (stdio/http, command, url, …) is intentionally not locked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerConfig {
    pub capability_description: String,
}

/// Explicit lookup/register failure — never silently return empty config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpServerLookupError {
    NotFound,
    InvalidKey,
}

/// Decision-level description for the L1 internal MCP `todo_task` surface.
/// Field-level transport / mount schema remains deferred (L2 tech-doc).
const SEEDED_TODO_CAPABILITY_DESCRIPTION: &str =
    "internal knowledge-mcp todo_task capability surface";

fn table() -> &'static Mutex<HashMap<String, McpServerConfig>> {
    static TABLE: OnceLock<Mutex<HashMap<String, McpServerConfig>>> = OnceLock::new();
    // Host-authoritative defaults must exist on first access (production Set path),
    // not only when tests call `seed_defaults`.
    TABLE.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert(
            SEEDED_BUSINESS_KEY.to_string(),
            McpServerConfig {
                capability_description: SEEDED_TODO_CAPABILITY_DESCRIPTION.to_string(),
            },
        );
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

/// Register a business key → MCP Server config mapping (Host-internal).
pub fn register(key: &str, config: McpServerConfig) -> Result<(), McpServerLookupError> {
    validate_key(key)?;
    if config.capability_description.trim().is_empty() {
        return Err(McpServerLookupError::InvalidKey);
    }
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
    let _ = register(
        SEEDED_BUSINESS_KEY,
        McpServerConfig {
            capability_description: SEEDED_TODO_CAPABILITY_DESCRIPTION.to_string(),
        },
    );
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
