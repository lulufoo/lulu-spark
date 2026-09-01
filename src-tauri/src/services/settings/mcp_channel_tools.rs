//! Per-channel MCP tool allowlist (`/mcp/workbench`, `/mcp/cursor_ide`, `/mcp/mobile`).
//!
//! Persists nested `{ channel: { notes: [], todo: [], knowledge: [] } }`. Legacy
//! flat arrays are migrated on read. UI commands still accept/return flat enabled lists.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::mcp_host::catalog::catalog_groups;

pub const MCP_CHANNELS: &[&str] = &["workbench", "cursor_ide", "mobile"];

/// Catalog tools that Settings may list, but only `/mcp/workbench` may enable or expose.
pub const WORKBENCH_ONLY_TOOLS: &[&str] = &["delete_note"];

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnabledByGroup {
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub todo: Vec<String>,
    #[serde(default)]
    pub knowledge: Vec<String>,
}

#[derive(Debug, Default, Clone)]
struct Stored {
    channels: BTreeMap<String, EnabledByGroup>,
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

fn catalog_by_group() -> Vec<(&'static str, HashSet<String>)> {
    catalog_groups()
        .into_iter()
        .map(|(id, tools)| {
            (
                id,
                tools.into_iter().map(|t| t.name).collect::<HashSet<_>>(),
            )
        })
        .collect()
}

fn group_for_tool(name: &str) -> Option<&'static str> {
    for (group_id, names) in catalog_by_group() {
        if names.contains(name) {
            return Some(group_id);
        }
    }
    None
}

fn classify_flat(names: impl IntoIterator<Item = String>, catalog: &HashSet<String>) -> EnabledByGroup {
    let mut groups = EnabledByGroup::default();
    for name in names {
        if !catalog.contains(&name) {
            continue;
        }
        match group_for_tool(&name) {
            Some("notes") => groups.notes.push(name),
            Some("todo") => groups.todo.push(name),
            Some("knowledge") => groups.knowledge.push(name),
            _ => {}
        }
    }
    groups.notes.sort();
    groups.todo.sort();
    groups.knowledge.sort();
    groups
}

fn default_groups_for_channel(channel: &str, catalog: &HashSet<String>) -> EnabledByGroup {
    let flat: Vec<String> = apply_channel_tool_policy(channel, catalog.clone())
        .into_iter()
        .collect();
    classify_flat(flat, catalog)
}

fn apply_channel_tool_policy(channel: &str, mut names: HashSet<String>) -> HashSet<String> {
    if channel != "workbench" {
        for tool in WORKBENCH_ONLY_TOOLS {
            names.remove(*tool);
        }
    }
    names
}

fn apply_group_policy(channel: &str, mut groups: EnabledByGroup) -> EnabledByGroup {
    if channel != "workbench" {
        groups
            .notes
            .retain(|name| !WORKBENCH_ONLY_TOOLS.contains(&name.as_str()));
    }
    groups.notes.sort();
    groups.todo.sort();
    groups.knowledge.sort();
    groups
}

fn filter_groups(groups: EnabledByGroup, catalog: &HashSet<String>) -> EnabledByGroup {
    let mut filtered = EnabledByGroup::default();
    for name in groups.notes {
        if catalog.contains(&name) {
            filtered.notes.push(name);
        }
    }
    for name in groups.todo {
        if catalog.contains(&name) {
            filtered.todo.push(name);
        }
    }
    for name in groups.knowledge {
        if catalog.contains(&name) {
            filtered.knowledge.push(name);
        }
    }
    filtered.notes.sort();
    filtered.todo.sort();
    filtered.knowledge.sort();
    filtered
}

fn groups_to_flat(groups: &EnabledByGroup) -> HashSet<String> {
    groups
        .notes
        .iter()
        .chain(groups.todo.iter())
        .chain(groups.knowledge.iter())
        .cloned()
        .collect()
}

fn normalize_channel_entry(value: &Value, catalog: &HashSet<String>) -> EnabledByGroup {
    if let Some(list) = value.as_array() {
        let names: Vec<String> = list
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        return classify_flat(names, catalog);
    }
    if let Ok(groups) = serde_json::from_value::<EnabledByGroup>(value.clone()) {
        return filter_groups(groups, catalog);
    }
    EnabledByGroup::default()
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
    let Ok(root) = serde_json::from_str::<Value>(&text) else {
        return Stored::default();
    };
    let Some(channels) = root.get("channels").and_then(|v| v.as_object()) else {
        return Stored::default();
    };
    let catalog = catalog_names();
    let mut stored = Stored::default();
    for (channel, entry) in channels {
        stored
            .channels
            .insert(channel.clone(), normalize_channel_entry(entry, &catalog));
    }
    stored
}

fn save_stored(stored: &Stored) -> Result<(), String> {
    let path = store_path()?;
    let channels: BTreeMap<String, EnabledByGroup> = stored.channels.clone();
    let value = json!({ "channels": channels });
    atomic_json::write_json(&path, &value)
}

fn enabled_groups_from_stored(
    stored: &Stored,
    channel: &str,
    catalog: &HashSet<String>,
) -> EnabledByGroup {
    let raw = match stored.channels.get(channel) {
        None => default_groups_for_channel(channel, catalog),
        Some(groups) => filter_groups(groups.clone(), catalog),
    };
    apply_group_policy(channel, raw)
}

/// Enabled API keys partitioned by business group for one MCP channel.
pub fn enabled_by_group(channel: &str) -> EnabledByGroup {
    let _guard = LOCK.lock().ok();
    if !is_channel(channel) {
        return EnabledByGroup::default();
    }
    let catalog = catalog_names();
    enabled_groups_from_stored(&load_stored(), channel, &catalog)
}

/// Enabled tool names for a channel. Unknown channel → empty.
pub fn enabled_names(channel: &str) -> HashSet<String> {
    apply_channel_tool_policy(channel, groups_to_flat(&enabled_by_group(channel)))
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
    let selected = apply_group_policy(channel, classify_flat(enabled, &catalog));
    let default = default_groups_for_channel(channel, &catalog);
    let mut stored = load_stored();
    if selected == default {
        stored.channels.remove(channel);
    } else {
        stored.channels.insert(channel.to_string(), selected);
    }
    save_stored(&stored)
}

pub fn snapshot() -> Result<Value, String> {
    let _guard = LOCK.lock().map_err(|e| e.to_string())?;
    let groups = catalog_groups();
    let catalog = catalog_names();
    let stored = load_stored();
    let mut enabled = serde_json::Map::new();
    let mut enabled_grouped = serde_json::Map::new();
    for channel in MCP_CHANNELS {
        let grouped = enabled_groups_from_stored(&stored, channel, &catalog);
        let mut names: Vec<String> = groups_to_flat(&grouped).into_iter().collect();
        names.sort();
        enabled.insert((*channel).to_string(), json!(names));
        enabled_grouped.insert(
            (*channel).to_string(),
            json!({
                "notes": grouped.notes,
                "todo": grouped.todo,
                "knowledge": grouped.knowledge,
            }),
        );
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
                    "knowledge" => "Knowledge",
                    other => other,
                },
                "tools": tools.iter().map(|tool| json!({
                    "name": tool.name,
                    "description": tool.description,
                })).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "enabled": enabled,
        "enabled_grouped": enabled_grouped,
    }))
}

#[cfg(test)]
#[path = "../../unit-tests/services/mcp_channel_tools.rs"]
mod tests;
