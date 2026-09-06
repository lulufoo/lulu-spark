//! Knowledge hide-pattern records: `{cache_dir}/knowledge-hide-patterns.json`.
//!
//! File missing → seed four whole-name regexes. Corrupt file → empty list, no overwrite.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_hex12;

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

fn file_path() -> Result<PathBuf, String> {
    paths::knowledge_hide_patterns_path().map_err(|e| format!("{e:?}"))
}

fn seed_file() -> HideFile {
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

fn new_record_id(existing: &[HidePatternRecord]) -> String {
    loop {
        let id = random_hex12();
        if existing.iter().all(|row| row.id != id) {
            return id;
        }
    }
}

fn save_unlocked(data: &HideFile) -> Result<(), String> {
    let path = file_path()?;
    let value = json!({
        "version": data.version,
        "patterns": data.patterns,
    });
    atomic_json::write_json(&path, &value)
}

fn load_unlocked() -> Result<HideFile, String> {
    let path = file_path()?;
    if !path.is_file() {
        let seeded = seed_file();
        save_unlocked(&seeded)?;
        return Ok(seeded);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    match serde_json::from_str::<HideFile>(&text) {
        Ok(data) => Ok(data),
        Err(_) => Ok(HideFile {
            version: 1,
            patterns: Vec::new(),
        }),
    }
}

fn validate_pattern(pattern: &str) -> Result<(), String> {
    let trimmed = pattern.trim();
    if trimmed.is_empty() {
        return Err("pattern is empty".into());
    }
    Regex::new(trimmed).map_err(|e| e.to_string())?;
    Ok(())
}

fn list_value(data: &HideFile) -> serde_json::Value {
    json!({ "patterns": data.patterns })
}

pub fn compiled_hide_regexes() -> Vec<Regex> {
    load_or_seed()
        .into_iter()
        .filter_map(|row| Regex::new(&row.pattern).ok())
        .collect()
}

pub fn name_is_hidden(name: &str, regexes: &[Regex]) -> bool {
    regexes.iter().any(|re| re.is_match(name))
}

pub fn load_or_seed() -> Vec<HidePatternRecord> {
    let _guard = WRITE_LOCK.lock().ok();
    load_unlocked().map(|f| f.patterns).unwrap_or_default()
}

pub fn list_json() -> Result<serde_json::Value, String> {
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let data = load_unlocked()?;
    Ok(list_value(&data))
}

pub fn add(pattern: &str) -> Result<serde_json::Value, String> {
    if let Err(err) = validate_pattern(pattern) {
        return Ok(json!({ "error": err, "_status": 400 }));
    }
    let trimmed = pattern.trim().to_string();
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let mut data = load_unlocked()?;
    if data.patterns.iter().any(|row| row.pattern == trimmed) {
        return Ok(json!({ "error": "duplicate pattern", "_status": 409 }));
    }
    data.version = 1;
    data.patterns.push(HidePatternRecord {
        id: new_record_id(&data.patterns),
        pattern: trimmed,
    });
    save_unlocked(&data)?;
    Ok(list_value(&data))
}

pub fn update(id: &str, pattern: &str) -> Result<serde_json::Value, String> {
    let id = id.trim();
    if id.is_empty() {
        return Ok(json!({ "error": "missing id", "_status": 400 }));
    }
    if let Err(err) = validate_pattern(pattern) {
        return Ok(json!({ "error": err, "_status": 400 }));
    }
    let trimmed = pattern.trim().to_string();
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let mut data = load_unlocked()?;
    if data
        .patterns
        .iter()
        .any(|row| row.pattern == trimmed && row.id != id)
    {
        return Ok(json!({ "error": "duplicate pattern", "_status": 409 }));
    }
    let Some(row) = data.patterns.iter_mut().find(|row| row.id == id) else {
        return Ok(json!({ "error": "pattern not found", "_status": 404 }));
    };
    row.pattern = trimmed;
    data.version = 1;
    save_unlocked(&data)?;
    Ok(list_value(&data))
}

pub fn remove(id: &str) -> Result<serde_json::Value, String> {
    let id = id.trim();
    if id.is_empty() {
        return Ok(json!({ "error": "missing id", "_status": 400 }));
    }
    let _guard = WRITE_LOCK.lock().map_err(|e| e.to_string())?;
    let mut data = load_unlocked()?;
    let before = data.patterns.len();
    data.patterns.retain(|row| row.id != id);
    if data.patterns.len() == before {
        return Ok(json!({ "error": "pattern not found", "_status": 404 }));
    }
    data.version = 1;
    save_unlocked(&data)?;
    Ok(list_value(&data))
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/hide_patterns.rs"]
mod tests;
