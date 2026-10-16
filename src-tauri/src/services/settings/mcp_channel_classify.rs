//! Classify flat/grouped MCP tool names and rewrite retired tool names.

use std::collections::HashSet;

use crate::mcp_host::catalog::catalog_groups;

use super::mcp_channel_tools::EnabledByGroup;

const DOC_CONTENT: &str = "get_doc_content";
const DOC_FILE: &str = "get_doc_file";

/// Tool names that existed before workspaces and projects, and what replaced them.
/// Saved settings and callers still using an old name keep working through this table.
const RETIRED_TOOLS: &[(&str, &str)] = &[
    ("search_notes", "search"),
    ("search_knowledge", "search"),
    ("search_document", "search"),
    ("get_note_content_by_id", "get_doc_content"),
    ("get_note_content", "get_doc_content"),
    ("get_knowledge_content", "get_doc_content"),
    ("get_note_digest_by_id", "get_doc_digest"),
    ("get_note_digest", "get_doc_digest"),
    ("get_note_file", "get_doc_file"),
    ("get_knowledge_file", "get_doc_file"),
    ("create_note", "create_doc"),
    ("update_note", "update_doc"),
    ("delete_note", "delete_doc"),
    ("list_notes_categories", "list_projects"),
    ("list_knowledge_categories", "list_projects"),
    ("create_notes_category", "create_project"),
    ("update_notes_category", "update_project"),
    ("delete_notes_category", "delete_project"),
];

fn push_unique(out: &mut Vec<String>, name: &str) {
    if !out.iter().any(|n| n == name) {
        out.push(name.to_string());
    }
}

fn current_name(name: &str) -> &str {
    RETIRED_TOOLS
        .iter()
        .find(|(old, _)| *old == name)
        .map_or(name, |(_, new)| *new)
}

pub fn rewrite_retired_tool_names(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        push_unique(&mut out, current_name(&name));
    }
    out
}

/// Desktop channels moved from `get_doc_content` to `get_doc_file`; mobile keeps the old tool.
pub fn migrate_content_tools_to_file(channel: &str, mut groups: EnabledByGroup) -> EnabledByGroup {
    if channel == "mobile" {
        return groups;
    }
    if groups.docs.iter().any(|n| n == DOC_CONTENT) {
        groups.docs.retain(|n| n != DOC_CONTENT);
        push_unique(&mut groups.docs, DOC_FILE);
    }
    groups
}

/// Saved grouped settings from before the `docs` group: `notes` and `knowledge` fold into
/// `docs`, and every retired tool name is rewritten.
pub fn migrate_retired_groups(
    docs: Vec<String>,
    notes: Vec<String>,
    knowledge: Vec<String>,
    global: Vec<String>,
) -> EnabledByGroup {
    EnabledByGroup {
        docs: rewrite_retired_tool_names(docs.into_iter().chain(notes).chain(knowledge)),
        global: rewrite_retired_tool_names(global),
    }
}

fn sort_groups(groups: &mut EnabledByGroup) {
    groups.docs.sort();
    groups.global.sort();
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
    for name in rewrite_retired_tool_names(names) {
        if !catalog.contains(&name) {
            continue;
        }
        match group_for_tool(&name) {
            Some("docs") => groups.docs.push(name),
            Some("global") => groups.global.push(name),
            _ => {}
        }
    }
    sort_groups(&mut groups);
    groups
}

pub fn filter_groups(groups: EnabledByGroup, catalog: &HashSet<String>) -> EnabledByGroup {
    let mut filtered = EnabledByGroup {
        docs: groups.docs.into_iter().filter(|n| catalog.contains(n)).collect(),
        global: groups.global.into_iter().filter(|n| catalog.contains(n)).collect(),
    };
    sort_groups(&mut filtered);
    filtered
}

pub fn groups_to_flat(groups: &EnabledByGroup) -> HashSet<String> {
    groups
        .docs
        .iter()
        .chain(groups.global.iter())
        .cloned()
        .collect()
}

pub fn sort_enabled_groups(groups: &mut EnabledByGroup) {
    sort_groups(groups);
}
