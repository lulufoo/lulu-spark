//! Per-channel MCP tool allowlist (`/mcp/workbench`, `/mcp/cursor_ide`, `/mcp/mobile`).
//!
//! Missing channel key = all catalog tools enabled (current Host behavior),
//! minus tools that are product-hard-gated to another channel.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::mcp_protocol_adapter::catalog_groups;

pub const MCP_CHANNELS: &[&str] = &["workbench", "cursor_ide", "mobile"];

/// Catalog tools that Settings may list, but only `/mcp/workbench` may enable or expose.
pub const WORKBENCH_ONLY_TOOLS: &[&str] = &["delete_note"];

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    channels: BTreeMap<String, Vec<String>>,
}

fn is_channel(name: &str) -> bool {
    MCP_CHANNELS.contains(&name)
}

fn catalog_names() -> HashSet<String> {
    catalog_groups()
        .into_iter()
        .flat_map(|(_, tools)| tools.into_iter().map(|t| t.name))
        .collect()
}

fn store_path() -> Result<std::path::PathBuf, String> {
    paths::mcp_channel_tools_path().map_err(|e| format!("{e:?}"))
}

fn load_stored() -> Stored {
    let Ok(path) = store_path() else {
        return Stored::default();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return Stored::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_stored(stored: &Stored) -> Result<(), String> {
    let path = store_path()?;
    let value = serde_json::to_value(stored).map_err(|e| e.to_string())?;
    atomic_json::write_json(&path, &value)
}

fn apply_channel_tool_policy(channel: &str, mut names: HashSet<String>) -> HashSet<String> {
    if channel != "workbench" {
        for tool in WORKBENCH_ONLY_TOOLS {
            names.remove(*tool);
        }
    }
    names
}

fn effective_catalog(channel: &str, catalog: &HashSet<String>) -> HashSet<String> {
    apply_channel_tool_policy(channel, catalog.clone())
}

fn enabled_from_stored(stored: &Stored, channel: &str, catalog: &HashSet<String>) -> HashSet<String> {
    let raw = match stored.channels.get(channel) {
        None => catalog.clone(),
        Some(list) => list
            .iter()
            .filter(|name| catalog.contains(*name))
            .cloned()
            .collect(),
    };
    apply_channel_tool_policy(channel, raw)
}

/// Enabled tool names for a channel. Unknown channel → empty.
pub fn enabled_names(channel: &str) -> HashSet<String> {
    let _guard = LOCK.lock().ok();
    if !is_channel(channel) {
        return HashSet::new();
    }
    let catalog = catalog_names();
    enabled_from_stored(&load_stored(), channel, &catalog)
}

pub fn is_enabled(channel: &str, name: &str) -> bool {
    enabled_names(channel).contains(name)
}

pub fn set_enabled(channel: &str, enabled: Vec<String>) -> Result<(), String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    if !is_channel(channel) {
        return Err(format!("unknown mcp channel: {channel}"));
    }
    let catalog = catalog_names();
    for name in &enabled {
        if !catalog.contains(name) {
            return Err(format!("unknown tool: {name}"));
        }
    }
    let mut selected: HashSet<String> = enabled.into_iter().collect();
    selected = apply_channel_tool_policy(channel, selected);
    let mut stored = load_stored();
    if selected == effective_catalog(channel, &catalog) {
        stored.channels.remove(channel);
    } else {
        let mut list: Vec<String> = selected.into_iter().collect();
        list.sort();
        stored.channels.insert(channel.to_string(), list);
    }
    save_stored(&stored)
}

pub fn snapshot() -> Result<Value, String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    let groups = catalog_groups();
    let catalog = catalog_names();
    let stored = load_stored();
    let mut enabled = serde_json::Map::new();
    for channel in MCP_CHANNELS {
        let mut names: Vec<String> = enabled_from_stored(&stored, channel, &catalog)
            .into_iter()
            .collect();
        names.sort();
        enabled.insert((*channel).to_string(), json!(names));
    }
    Ok(json!({
        "channels": MCP_CHANNELS,
        "workbench_only_tools": WORKBENCH_ONLY_TOOLS,
        "groups": groups.iter().map(|(id, tools)| {
            json!({
                "id": id,
                "label": match *id {
                    "notes" => "Notes",
                    "todo" => "Todo",
                    other => other,
                },
                "tools": tools.iter().map(|tool| json!({
                    "name": tool.name,
                    "description": tool.description,
                })).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "enabled": enabled,
    }))
}

#[cfg(test)]
#[path = "../unit-tests/services/mcp_channel_tools.rs"]
mod tests;
