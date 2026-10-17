//! Document classification list (`source_type`): builtin JSON plus a local copy.
//!
//! Missing local file → write the builtin default, then return it. Later edits
//! only change the local file. Not a sqlite table.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::paths;
use crate::repositories::atomic_json;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

const BUILTIN_JSON: &str = include_str!("../../../resources/doc_categories.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocCategory {
    pub id: String,
    pub label: String,
    pub exposed: bool,
}

pub fn builtin_categories() -> Vec<DocCategory> {
    parse_list(BUILTIN_JSON).expect("builtin doc_categories.json")
}

fn file_path() -> Result<std::path::PathBuf, String> {
    paths::doc_categories_path().map_err(|e| format!("{e:?}"))
}

fn parse_list(text: &str) -> Result<Vec<DocCategory>, String> {
    let raw: Value =
        serde_json::from_str(text).map_err(|e| format!("invalid doc categories JSON: {e}"))?;
    let Value::Array(items) = raw else {
        return Err("doc categories JSON must be an array".into());
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let obj = item
            .as_object()
            .ok_or_else(|| "each category must be an object".to_string())?;
        let id = string_field(obj, "id")?;
        let label = string_field(obj, "label")?;
        let exposed = match obj.get("exposed") {
            Some(Value::Bool(v)) => *v,
            Some(_) => return Err("category exposed must be a boolean".into()),
            None => return Err("category exposed is required".into()),
        };
        out.push(DocCategory { id, label, exposed });
    }
    validate(&out)?;
    Ok(out)
}

fn string_field(obj: &serde_json::Map<String, Value>, key: &str) -> Result<String, String> {
    match obj.get(key) {
        Some(Value::String(s)) => {
            let t = s.trim().to_string();
            if t.is_empty() {
                Err(format!("category {key} is empty"))
            } else {
                Ok(t)
            }
        }
        Some(_) => Err(format!("category {key} must be a string")),
        None => Err(format!("category {key} is required")),
    }
}

/// Lowercase letters and digits, hyphen-separated. No spaces, no leading/trailing hyphen.
fn valid_id_form(id: &str) -> bool {
    let mut bytes = id.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    let mut prev_hyphen = false;
    for b in bytes {
        if b == b'-' {
            if prev_hyphen {
                return false;
            }
            prev_hyphen = true;
            continue;
        }
        prev_hyphen = false;
        if !b.is_ascii_lowercase() && !b.is_ascii_digit() {
            return false;
        }
    }
    !prev_hyphen
}

fn validate(items: &[DocCategory]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for item in items {
        if item.id.trim().is_empty() {
            return Err("category id is empty".into());
        }
        if item.id.chars().any(char::is_whitespace) {
            return Err("category id cannot contain whitespace".into());
        }
        if !valid_id_form(&item.id) {
            return Err("category id must be lowercase letters, digits, and hyphens".into());
        }
        if item.label.trim().is_empty() {
            return Err("category label is empty".into());
        }
        if !seen.insert(item.id.clone()) {
            return Err(format!("duplicate category id: {}", item.id));
        }
    }
    Ok(())
}

fn write_items(path: &Path, items: &[DocCategory]) -> Result<(), String> {
    let value = serde_json::to_value(items).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

fn read_unlocked() -> Result<Vec<DocCategory>, String> {
    let path = file_path()?;
    if !path.is_file() {
        return Ok(builtin_categories());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    parse_list(&text)
}

fn load_unlocked() -> Result<Vec<DocCategory>, String> {
    let path = file_path()?;
    if !path.is_file() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let items = builtin_categories();
        write_items(&path, &items)?;
        return Ok(items);
    }
    read_unlocked()
}

/// Local list. Missing file → write builtin default first.
pub fn load() -> Result<Vec<DocCategory>, String> {
    let _guard = WRITE_LOCK.lock().ok();
    load_unlocked()
}

/// Replace the whole local list after validation.
pub fn save(items: &[DocCategory]) -> Result<(), String> {
    validate(items)?;
    let _guard = WRITE_LOCK.lock().ok();
    let path = file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    write_items(&path, items)
}

pub fn is_legal(id: &str) -> bool {
    let id = id.trim();
    load().ok().is_some_and(|items| items.iter().any(|c| c.id == id))
}

pub fn is_exposed(id: &str) -> bool {
    let id = id.trim();
    load()
        .ok()
        .is_some_and(|items| items.iter().any(|c| c.id == id && c.exposed))
}

/// Exposed ids for MCP schema. Missing local file uses builtin and does not write.
pub fn exposed_ids() -> Vec<String> {
    let _guard = WRITE_LOCK.lock().ok();
    match read_unlocked() {
        Ok(items) => items
            .into_iter()
            .filter(|c| c.exposed)
            .map(|c| c.id)
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_categories.rs"]
mod tests;
