//! Hidden files of one workspace: filename regexes kept in
//! `{cache_dir}/workspace-hide-patterns/{workspace_id}.json`.
//!
//! A document is hidden when any segment of its `raw_path` matches any rule. A hidden document
//! drops out of the workspace's index (tree, counts, timeline) and out of search collect; it
//! stays readable by id, like a disabled project's documents.
//!
//! First read of a workspace with no file seeds the rules from the single global list that
//! existed before rules were per workspace (`knowledge-hide-patterns.json`) so existing
//! settings are kept; with neither file it seeds four default rules. A corrupt file reads as
//! an empty list and is never overwritten.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_hex12;
use crate::services::spark_data::validate_workspace_id;

use super::resolve::{bad_request, conflict, internal, not_found, open_data, resolve_workspace, text_field};

static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub const SEED_PATTERNS: &[&str] = &[
    r"^\.git$",
    r"^\.cache$",
    r"^\.worktrees$",
    r"^\.repository-type\.json$",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HidePatternRecord {
    pub id: String,
    pub pattern: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct HideFile {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default)]
    patterns: Vec<HidePatternRecord>,
}

fn default_version() -> u32 {
    1
}

fn file_path(workspace_id: &str) -> Result<PathBuf, String> {
    validate_workspace_id(workspace_id)?;
    paths::workspace_hide_patterns_path(workspace_id).map_err(|e| format!("{e:?}"))
}

fn default_seed() -> HideFile {
    HideFile {
        version: 1,
        patterns: SEED_PATTERNS
            .iter()
            .enumerate()
            .map(|(i, pattern)| HidePatternRecord {
                id: format!("seed{i:02}"),
                pattern: (*pattern).to_string(),
            })
            .collect(),
    }
}

/// The global list from before rules were per workspace, when it exists and parses.
fn legacy_seed() -> Option<HideFile> {
    let path = paths::legacy_hide_patterns_path().ok()?;
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<HideFile>(&text).ok()
}

fn new_record_id(existing: &[HidePatternRecord]) -> String {
    loop {
        let id = random_hex12();
        if existing.iter().all(|row| row.id != id) {
            return id;
        }
    }
}

fn save_unlocked(workspace_id: &str, data: &HideFile) -> Result<(), String> {
    let value = json!({ "version": data.version, "patterns": data.patterns });
    atomic_json::write_json(&file_path(workspace_id)?, &value)
}

fn load_unlocked(workspace_id: &str) -> Result<HideFile, String> {
    let path = file_path(workspace_id)?;
    if !path.is_file() {
        let seeded = legacy_seed().unwrap_or_else(default_seed);
        save_unlocked(workspace_id, &seeded)?;
        return Ok(seeded);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str::<HideFile>(&text).unwrap_or(HideFile {
        version: 1,
        patterns: Vec::new(),
    }))
}

fn validate_pattern(pattern: &str) -> Result<(), String> {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return Err("pattern is empty".into());
    }
    Regex::new(trimmed).map(|_| ()).map_err(|e| e.to_string())
}

fn list_value(data: &HideFile) -> Value {
    json!({ "patterns": data.patterns })
}

/// The rules of `workspace_id`, compiled. A rule that does not compile is skipped; a workspace
/// whose rules cannot be read hides nothing.
pub fn compiled_hide_regexes(workspace_id: &str) -> Vec<Regex> {
    let _guard = WRITE_LOCK.lock().ok();
    load_unlocked(workspace_id)
        .map(|file| file.patterns)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| Regex::new(&row.pattern).ok())
        .collect()
}

/// True when any `/`-separated segment of `raw_path` matches any rule.
pub fn path_is_hidden(raw_path: &str, regexes: &[Regex]) -> bool {
    !regexes.is_empty()
        && raw_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .any(|segment| regexes.iter().any(|re| re.is_match(segment)))
}

/// `payload.workspace_id` (default workspace when absent) resolved against the registry.
fn workspace_id_of(payload: &Value) -> Result<String, Value> {
    let conn = open_data()?;
    resolve_workspace(&conn, text_field(payload, "workspace_id")).map(|ws| ws.id)
}

/// `{ workspace_id? }` → `{ patterns: [{ id, pattern }] }`.
pub fn list_hide_patterns(payload: &Value) -> Value {
    let id = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let _guard = WRITE_LOCK.lock().ok();
    match load_unlocked(&id) {
        Ok(data) => list_value(&data),
        Err(e) => internal(e),
    }
}

/// `{ workspace_id?, pattern }` → the updated list. 400 for an empty or invalid regex, 409 for a duplicate.
pub fn add_hide_pattern(payload: &Value) -> Value {
    let id = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let pattern = text_field(payload, "pattern").unwrap_or("");
    if let Err(err) = validate_pattern(pattern) {
        return bad_request(err);
    }
    let _guard = WRITE_LOCK.lock().ok();
    let mut data = match load_unlocked(&id) {
        Ok(data) => data,
        Err(e) => return internal(e),
    };
    if data.patterns.iter().any(|row| row.pattern == pattern) {
        return conflict("duplicate pattern");
    }
    data.version = 1;
    data.patterns.push(HidePatternRecord {
        id: new_record_id(&data.patterns),
        pattern: pattern.to_string(),
    });
    match save_unlocked(&id, &data) {
        Ok(()) => list_value(&data),
        Err(e) => internal(e),
    }
}

/// `{ workspace_id?, id, pattern }` → the updated list.
pub fn update_hide_pattern(payload: &Value) -> Value {
    let workspace = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let Some(id) = text_field(payload, "id") else {
        return bad_request("missing id");
    };
    let pattern = text_field(payload, "pattern").unwrap_or("");
    if let Err(err) = validate_pattern(pattern) {
        return bad_request(err);
    }
    let _guard = WRITE_LOCK.lock().ok();
    let mut data = match load_unlocked(&workspace) {
        Ok(data) => data,
        Err(e) => return internal(e),
    };
    if data.patterns.iter().any(|row| row.pattern == pattern && row.id != id) {
        return conflict("duplicate pattern");
    }
    let Some(row) = data.patterns.iter_mut().find(|row| row.id == id) else {
        return not_found("pattern not found");
    };
    row.pattern = pattern.to_string();
    data.version = 1;
    match save_unlocked(&workspace, &data) {
        Ok(()) => list_value(&data),
        Err(e) => internal(e),
    }
}

/// `{ workspace_id?, id }` → the updated list.
pub fn remove_hide_pattern(payload: &Value) -> Value {
    let workspace = match workspace_id_of(payload) {
        Ok(id) => id,
        Err(v) => return v,
    };
    let Some(id) = text_field(payload, "id") else {
        return bad_request("missing id");
    };
    let _guard = WRITE_LOCK.lock().ok();
    let mut data = match load_unlocked(&workspace) {
        Ok(data) => data,
        Err(e) => return internal(e),
    };
    let before = data.patterns.len();
    data.patterns.retain(|row| row.id != id);
    if data.patterns.len() == before {
        return not_found("pattern not found");
    }
    data.version = 1;
    match save_unlocked(&workspace, &data) {
        Ok(()) => list_value(&data),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_hide_patterns.rs"]
mod tests;
