//! Host-facing read interface for Settings MCP catalog state.
//!
//! Persistence and UI get/set stay in [`super::mcp_channel_tools`].
//! Host MCP runtime reads grouped enabled lists through this module only.

use std::collections::HashSet;

use super::mcp_channel_tools;

/// Enabled API keys per business group (same shape as the saved Settings entry).
pub type GroupedEnabledCatalog = mcp_channel_tools::EnabledByGroup;

/// Enabled API keys for `channel`, partitioned by business group.
pub fn enabled_grouped(channel: &str) -> GroupedEnabledCatalog {
    mcp_channel_tools::enabled_by_group(channel)
}

/// Flat enabled names (settings storage view). Prefer [`enabled_grouped`] for MCP list build.
pub fn enabled_names(channel: &str) -> HashSet<String> {
    mcp_channel_tools::enabled_names(channel)
}

pub fn is_enabled(channel: &str, name: &str) -> bool {
    mcp_channel_tools::is_enabled(channel, name)
}

#[cfg(test)]
#[path = "../../unit-tests/services/settings_mcp_catalog.rs"]
mod tests;
