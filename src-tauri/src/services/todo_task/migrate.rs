//! v2 bootstrap leftovers: implicit-sub migration and the durable HTTP gate.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;

use super::categories::ensure_default_category_unlocked;
use super::error::{bootstrap_error, TodoError};
use super::store::{
    ensure_bootstrap, load_v2_index_unlocked, with_write_lock, write_sub_tasks_json,
};
use super::types::{SubTasksFile, DEFAULT_CATEGORY_ID};

#[cfg(test)]
use super::test_hooks::test_take_fail_migrate_implicit_write;

pub(super) fn migration_errors_path() -> Result<std::path::PathBuf, String> {
    paths::todo_tasks_dir()
        .map(|d| d.join("migration_errors.json"))
        .map_err(|e| format!("{e:?}"))
}

pub(super) fn load_migration_errors_unlocked() -> HashMap<String, String> {
    let Ok(path) = migration_errors_path() else {
        return HashMap::new();
    };
    if !path.is_file() {
        return HashMap::new();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub(super) fn save_migration_errors(errors: &HashMap<String, String>) {
    let Ok(path) = migration_errors_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if errors.is_empty() {
        if path.is_file() {
            let _ = fs::remove_file(&path);
        }
        return;
    }
    if let Ok(text) = serde_json::to_string_pretty(errors) {
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &text).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }
}

pub(super) fn plan_has_migration_error(master_id: &str) -> bool {
    load_migration_errors_unlocked().contains_key(master_id)
}

pub(super) fn write_migrated_sub_tasks(path: &Path, sub_tasks: &SubTasksFile) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_migrate_implicit_write() {
        return Err("injected migrate_implicit_subs write failure".to_string());
    }
    write_sub_tasks_json(path, sub_tasks)
}

pub(super) fn migrate_one_plan_implicit_subs(master_id: &str) -> Result<(), String> {
    let path = paths::todo_tasks_sub_tasks_path(master_id).map_err(|e| format!("{e:?}"))?;
    if !path.is_file() {
        return Err("missing sub_tasks.json".to_string());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut file: SubTasksFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let before = file.sub_tasks.len();
    file.sub_tasks.retain(|s| !s.implicit);
    if file.sub_tasks.len() == before {
        return Ok(());
    }
    write_migrated_sub_tasks(&path, &file)
}

pub(super) fn migrate_implicit_subs() {
    let index = match load_v2_index_unlocked() {
        Ok(i) => i,
        Err(_) => return,
    };
    let mut errors = load_migration_errors_unlocked();
    for master_id in index.tasks.keys() {
        match migrate_one_plan_implicit_subs(master_id) {
            Ok(()) => {
                errors.remove(master_id);
            }
            Err(message) => {
                errors.insert(master_id.clone(), message);
            }
        }
    }
    save_migration_errors(&errors);
}

/// One-shot migration: stamp missing/empty `category_id` on disk to default「待分类」.
pub fn migrate_todos_default_category() -> Result<Value, TodoError> {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }
        if let Err(e) = ensure_default_category_unlocked() {
            return Err(TodoError::internal(e));
        }
        let index_path = match paths::todo_tasks_index_path() {
            Ok(p) => p,
            Err(e) => return Err(TodoError::internal(format!("{e:?}"))),
        };
        if !index_path.is_file() {
            return Ok(json!({ "updated": 0 }));
        }
        let text = match fs::read_to_string(&index_path) {
            Ok(t) => t,
            Err(e) => return Err(TodoError::internal(e.to_string())),
        };
        let mut root: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => return Err(TodoError::internal(e.to_string())),
        };
        let Some(tasks) = root.get_mut("tasks").and_then(|t| t.as_object_mut()) else {
            return Ok(json!({ "updated": 0 }));
        };
        let mut updated = 0u64;
        for (_id, entry) in tasks.iter_mut() {
            let needs = match entry.get("category_id") {
                None => true,
                Some(Value::Null) => true,
                Some(Value::String(s)) if s.trim().is_empty() => true,
                Some(Value::String(_)) => false,
                Some(_) => true,
            };
            if needs {
                if let Some(obj) = entry.as_object_mut() {
                    obj.insert(
                        "category_id".to_string(),
                        json!(DEFAULT_CATEGORY_ID),
                    );
                    updated += 1;
                }
            }
        }
        if updated > 0 {
            if let Err(e) = atomic_json::write_json(&index_path, &root) {
                return Err(TodoError::internal(e));
            }
        }
        Ok(json!({ "updated": updated }))
    })
}

/// Durable migration gate marker written by the standalone migrate script on full success.
pub const MIGRATION_GATE_FILE: &str = ".migration_gate_passed";

/// True when `todo_tasks/.migration_gate_passed` exists on disk.
/// Re-reads the filesystem each call (cold restart / no process-local script exit code).
pub fn migration_gate_passed() -> bool {
    match paths::todo_tasks_dir() {
        Ok(dir) => dir.join(MIGRATION_GATE_FILE).is_file(),
        Err(_) => false,
    }
}

/// Open todo HTTP only when the durable migration gate marker is present.
pub fn ensure_todo_api_ungated() -> Result<(), TodoError> {
    if migration_gate_passed() {
        Ok(())
    } else {
        Err(TodoError::with_status(
            503,
            "Todo API gated: missing todo_tasks/.migration_gate_passed",
        ))
    }
}
