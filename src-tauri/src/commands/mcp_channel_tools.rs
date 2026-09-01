//! Settings commands for per-channel MCP tool checkboxes.

use serde_json::{json, Value};

#[tauri::command]
pub fn get_mcp_channel_tools() -> Result<Value, String> {
    crate::services::settings::mcp_channel_tools::snapshot()
}

#[tauri::command]
pub fn set_mcp_channel_tools(channel: String, enabled: Vec<String>) -> Result<Value, String> {
    crate::services::settings::mcp_channel_tools::set_enabled(&channel, enabled)?;
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
#[path = "../unit-tests/commands/mcp_channel_tools.rs"]
mod tests;
