//! Scene slot registration and catalog tool tables.
//!
//! `scene_slot` is HTTP/auth only. Which tools exist comes from Settings grouped
//! enabled lists + catalog factory — not from per-slot business flags.

use super::types::{SlotToolTable, ToolDescriptor, REGISTERED_SCENE_SLOTS};
use crate::mcp_host::catalog::groups::{notes, todo};
use crate::mcp_host::catalog::{build_routes_for_channel, catalog_groups};
use crate::services::settings::mcp_catalog;

pub(crate) fn is_registered_scene_slot(slot: &str) -> bool {
    REGISTERED_SCENE_SLOTS.contains(&slot)
}

/// Full catalog snapshot for contract checks (all groups; not Settings-enabled subset).
pub fn build_slot_tool_table(slot: &str) -> Option<SlotToolTable> {
    if !is_registered_scene_slot(slot) {
        return None;
    }
    let tools = catalog_groups()
        .into_iter()
        .flat_map(|(_, routes)| routes)
        .collect();
    Some(SlotToolTable {
        scene_slot: slot.to_string(),
        tools,
    })
}

/// Allowlisted tool names for a slot (empty when unregistered).
pub fn tools_list_for_slot(slot: &str) -> Vec<ToolDescriptor> {
    build_slot_tool_table(slot)
        .map(|table| {
            table
                .tools
                .into_iter()
                .map(|t| ToolDescriptor { name: t.name })
                .collect()
        })
        .unwrap_or_default()
}

/// All catalog route definitions for one MCP channel (parity / contract tests).
pub fn build_channel_tool_table(slot: &str, channel: &str) -> Option<SlotToolTable> {
    if !is_registered_scene_slot(slot) {
        return None;
    }
    let mut tools = Vec::new();
    tools.extend(notes::routes_for_channel(channel));
    tools.extend(todo::routes_for_channel(channel));
    Some(SlotToolTable {
        scene_slot: slot.to_string(),
        tools,
    })
}

/// Runtime tool table: Settings grouped enabled → catalog factory.
pub fn build_channel_enabled_table(slot: &str, channel: &str) -> Option<SlotToolTable> {
    if !is_registered_scene_slot(slot) {
        return None;
    }
    let enabled = mcp_catalog::enabled_grouped(channel);
    let tools = build_routes_for_channel(slot, channel, &enabled);
    Some(SlotToolTable {
        scene_slot: slot.to_string(),
        tools,
    })
}
