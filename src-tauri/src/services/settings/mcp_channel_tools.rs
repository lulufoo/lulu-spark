//! Per-channel MCP tool allowlist (`/mcp/spark`, `/mcp/cursor`, `/mcp/codex`,
//! `/mcp/claude`, `/mcp/mobile`). `cursor_ide` is a legacy alias of `cursor`.
//!
//! Persists nested `{ channel: { global: [], workspace: [], project: [], document: [], read_later: [] } }`.
//! Legacy flat arrays and the older `docs` group (workspace, project and document tools in one
//! list) are re-sorted into the five groups on read; nothing is lost and nothing is added.
//! UI commands still accept/return flat enabled lists.
//! Retired tool names (`create_note`, `search_notes`, ...) rewrite to their replacement.
//! Tools a channel does not expose (per the tool's own `build`) are never enabled there;
//! on desktop channels a saved `get_doc_content` entry migrates to `get_doc_file`.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::mcp_host::catalog::{catalog_groups, factory};
use super::mcp_channel_classify::{
    classify_flat, filter_groups, groups_to_flat, migrate_content_tools_to_file,
    sort_enabled_groups,
};

pub const MCP_CHANNELS: &[&str] = &["spark", "cursor", "codex", "claude", "mobile"];

fn canonical_channel(name: &str) -> Option<&'static str> {
    match name {
        "spark" => Some("spark"),
        "cursor" | "cursor_ide" => Some("cursor"),
        "codex" => Some("codex"),
        "claude" => Some("claude"),
        "mobile" => Some("mobile"),
        _ => None,
    }
}

/// Catalog tools that Settings may list, but only `/mcp/spark` may enable or expose.
pub const SPARK_ONLY_TOOLS: &[&str] = &["delete_doc"];

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnabledByGroup {
    #[serde(default)]
    pub global: Vec<String>,
    #[serde(default)]
    pub workspace: Vec<String>,
    #[serde(default)]
    pub project: Vec<String>,
    #[serde(default)]
    pub document: Vec<String>,
    #[serde(default)]
    pub read_later: Vec<String>,
}

/// Group ids that may appear in a saved entry. `docs` is the pre-split group.
const SAVED_GROUP_KEYS: &[&str] = &[
    "docs",
    "global",
    "workspace",
    "project",
    "document",
    "read_later",
];

impl EnabledByGroup {
    /// `(group id, names)` for the five groups, in Settings order.
    pub fn groups(&self) -> [(&'static str, &Vec<String>); 5] {
        [
            ("global", &self.global),
            ("workspace", &self.workspace),
            ("project", &self.project),
            ("document", &self.document),
            ("read_later", &self.read_later),
        ]
    }

    pub fn groups_mut(&mut self) -> [(&'static str, &mut Vec<String>); 5] {
        [
            ("global", &mut self.global),
            ("workspace", &mut self.workspace),
            ("project", &mut self.project),
            ("document", &mut self.document),
            ("read_later", &mut self.read_later),
        ]
    }

    pub fn push(&mut self, group: &str, name: String) {
        if let Some((_, list)) = self.groups_mut().into_iter().find(|(id, _)| *id == group) {
            list.push(name);
        }
    }
}

#[derive(Debug, Default, Clone)]
struct Stored {
    channels: BTreeMap<String, EnabledByGroup>,
}

fn catalog_names() -> HashSet<String> {
    catalog_groups()
        .into_iter()
        .flat_map(|(_, tools)| tools.into_iter().map(|t| t.name))
        .collect()
}

fn default_groups_for_channel(channel: &str, catalog: &HashSet<String>) -> EnabledByGroup {
    let flat: Vec<String> = apply_channel_tool_policy(channel, catalog.clone())
        .into_iter()
        .collect();
    classify_flat(flat, catalog)
}

/// True when the tool's own `build` does not expose it on `channel`.
fn hidden_on_channel(channel: &str, name: &str) -> bool {
    factory::group_for_migrated_api(name)
        .is_some_and(|group| factory::build(group, name, channel).is_none())
}

fn apply_channel_tool_policy(channel: &str, mut names: HashSet<String>) -> HashSet<String> {
    if channel != "spark" {
        for tool in SPARK_ONLY_TOOLS {
            names.remove(*tool);
        }
    }
    names.retain(|name| !hidden_on_channel(channel, name));
    names
}

fn apply_group_policy(channel: &str, mut groups: EnabledByGroup) -> EnabledByGroup {
    if channel != "spark" {
        for (_, names) in groups.groups_mut() {
            names.retain(|name| !SPARK_ONLY_TOOLS.contains(&name.as_str()));
        }
    }
    for (_, names) in groups.groups_mut() {
        names.retain(|name| !hidden_on_channel(channel, name));
    }
    sort_enabled_groups(&mut groups);
    groups
}

fn normalize_channel_entry(value: &Value, catalog: &HashSet<String>) -> EnabledByGroup {
    if let Some(list) = value.as_array() {
        let names: Vec<String> = list
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        return classify_flat(names, catalog);
    }
    let Some(object) = value.as_object() else {
        return EnabledByGroup::default();
    };
    let names = |key: &str| -> Vec<String> {
        object
            .get(key)
            .and_then(Value::as_array)
            .map(|list| list.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };
    // A saved entry may still list workspace/project/document tools under the old `docs` key, so
    // every tool is placed again by the group it belongs to now.
    let saved: Vec<String> = SAVED_GROUP_KEYS.iter().flat_map(|key| names(key)).collect();
    classify_flat(saved, catalog)
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
        let Some(canon) = canonical_channel(channel) else {
            continue;
        };
        let groups = migrate_content_tools_to_file(canon, normalize_channel_entry(entry, &catalog));
        stored.channels.insert(canon.to_string(), groups);
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
    let Some(channel) = canonical_channel(channel) else {
        return EnabledByGroup::default();
    };
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
    let Some(channel) = canonical_channel(channel) else {
        return Err(format!("unknown mcp channel: {channel}"));
    };
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
    let mut unavailable = serde_json::Map::new();
    for channel in MCP_CHANNELS {
        let mut hidden: Vec<&String> = catalog
            .iter()
            .filter(|name| hidden_on_channel(channel, name))
            .collect();
        hidden.sort();
        unavailable.insert((*channel).to_string(), json!(hidden));
        let grouped = enabled_groups_from_stored(&stored, channel, &catalog);
        let mut names: Vec<String> = groups_to_flat(&grouped).into_iter().collect();
        names.sort();
        enabled.insert((*channel).to_string(), json!(names));
        enabled_grouped.insert(
            (*channel).to_string(),
            json!({
                "global": grouped.global,
                "workspace": grouped.workspace,
                "project": grouped.project,
                "document": grouped.document,
                "read_later": grouped.read_later,
            }),
        );
    }
    Ok(json!({
        "channels": MCP_CHANNELS,
        "spark_only_tools": SPARK_ONLY_TOOLS,
        "groups": groups.iter().map(|(id, tools)| {
            json!({
                "id": id,
                "label": match *id {
                    "global" => "Global",
                    "workspace" => "Workspace",
                    "project" => "Project",
                    "document" => "Document",
                    "read_later" => "Read Later",
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
        "unavailable_by_channel": unavailable,
    }))
}

#[cfg(test)]
#[path = "../../unit-tests/services/mcp_channel_tools.rs"]
mod tests;
