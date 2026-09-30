//! Classify flat/grouped MCP tool names and rewrite retired search APIs.

use std::collections::HashSet;

use crate::mcp_host::catalog::catalog_groups;

use super::mcp_channel_tools::EnabledByGroup;

const RETIRED_SEARCH_TOOLS: &[&str] = &["search_notes", "search_knowledge"];
const SEARCH_DOCUMENT: &str = "search_document";
const RETIRED_NOTE_CONTENT: &str = "get_note_content_by_id";
const NOTE_CONTENT: &str = "get_note_content";

fn push_unique(out: &mut Vec<String>, name: &str) {
    if !out.iter().any(|n| n == name) {
        out.push(name.to_string());
    }
}

pub fn rewrite_retired_search_names(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut add_document = false;
    for name in names {
        if RETIRED_SEARCH_TOOLS.contains(&name.as_str()) {
            add_document = true;
            continue;
        }
        if name == RETIRED_NOTE_CONTENT {
            push_unique(&mut out, NOTE_CONTENT);
            continue;
        }
        out.push(name);
    }
    if add_document {
        push_unique(&mut out, SEARCH_DOCUMENT);
    }
    out
}

pub fn migrate_retired_search_groups(mut groups: EnabledByGroup) -> EnabledByGroup {
    let had_old = groups.notes.iter().any(|n| n == "search_notes")
        || groups.knowledge.iter().any(|n| n == "search_knowledge");
    groups.notes.retain(|n| n != "search_notes");
    groups.knowledge.retain(|n| n != "search_knowledge");
    if had_old {
        push_unique(&mut groups.global, SEARCH_DOCUMENT);
    }
    let had_old_note = groups.notes.iter().any(|n| n == RETIRED_NOTE_CONTENT);
    groups.notes.retain(|n| n != RETIRED_NOTE_CONTENT);
    if had_old_note {
        push_unique(&mut groups.notes, NOTE_CONTENT);
    }
    groups
}

fn sort_groups(groups: &mut EnabledByGroup) {
    groups.notes.sort();
    groups.knowledge.sort();
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
    for name in rewrite_retired_search_names(names) {
        if !catalog.contains(&name) {
            continue;
        }
        match group_for_tool(&name) {
            Some("notes") => groups.notes.push(name),
            Some("knowledge") => groups.knowledge.push(name),
            Some("global") => groups.global.push(name),
            _ => {}
        }
    }
    sort_groups(&mut groups);
    groups
}

pub fn filter_groups(groups: EnabledByGroup, catalog: &HashSet<String>) -> EnabledByGroup {
    let mut filtered = EnabledByGroup::default();
    for name in groups.notes {
        if catalog.contains(&name) {
            filtered.notes.push(name);
        }
    }
    for name in groups.knowledge {
        if catalog.contains(&name) {
            filtered.knowledge.push(name);
        }
    }
    for name in groups.global {
        if catalog.contains(&name) {
            filtered.global.push(name);
        }
    }
    sort_groups(&mut filtered);
    filtered
}

pub fn groups_to_flat(groups: &EnabledByGroup) -> HashSet<String> {
    groups
        .notes
        .iter()
        .chain(groups.knowledge.iter())
        .chain(groups.global.iter())
        .cloned()
        .collect()
}

pub fn sort_enabled_groups(groups: &mut EnabledByGroup) {
    sort_groups(groups);
}
