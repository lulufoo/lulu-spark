//! Classify flat/grouped MCP tool names into their business groups.

use std::collections::HashSet;

use crate::mcp_host::catalog::catalog_groups;

use super::mcp_channel_tools::EnabledByGroup;

const DOC_CONTENT: &str = "get_doc_content";
const DOC_FILE: &str = "get_doc_file";

fn push_unique(out: &mut Vec<String>, name: &str) {
    if !out.iter().any(|n| n == name) {
        out.push(name.to_string());
    }
}

/// Desktop channels moved from `get_doc_content` to `get_doc_file`; mobile keeps the old tool.
pub fn migrate_content_tools_to_file(channel: &str, mut groups: EnabledByGroup) -> EnabledByGroup {
    if channel == "mobile" {
        return groups;
    }
    if groups.document.iter().any(|n| n == DOC_CONTENT) {
        groups.document.retain(|n| n != DOC_CONTENT);
        push_unique(&mut groups.document, DOC_FILE);
    }
    groups
}

fn sort_groups(groups: &mut EnabledByGroup) {
    for (_, names) in groups.groups_mut() {
        names.sort();
        names.dedup();
    }
}

fn group_for_tool(name: &str) -> Option<&'static str> {
    for (group_id, tools) in catalog_groups() {
        if tools.iter().any(|t| t.name == name) {
            return Some(group_id);
        }
    }
    None
}

pub fn classify_flat(
    names: impl IntoIterator<Item = String>,
    catalog: &HashSet<String>,
) -> EnabledByGroup {
    let mut groups = EnabledByGroup::default();
    for name in names {
        if !catalog.contains(&name) {
            continue;
        }
        if let Some(group) = group_for_tool(&name) {
            groups.push(group, name);
        }
    }
    sort_groups(&mut groups);
    groups
}

pub fn filter_groups(groups: EnabledByGroup, catalog: &HashSet<String>) -> EnabledByGroup {
    let mut filtered = groups;
    for (_, names) in filtered.groups_mut() {
        names.retain(|n| catalog.contains(n));
    }
    sort_groups(&mut filtered);
    filtered
}

pub fn groups_to_flat(groups: &EnabledByGroup) -> HashSet<String> {
    groups
        .groups()
        .into_iter()
        .flat_map(|(_, names)| names.iter().cloned())
        .collect()
}

pub fn sort_enabled_groups(groups: &mut EnabledByGroup) {
    sort_groups(groups);
}
