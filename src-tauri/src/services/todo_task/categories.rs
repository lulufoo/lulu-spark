//! Category registry and master category assignment.

use std::fs;

use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_hex12;

use super::error::{bootstrap_error, corrupt_storage_error, TodoError};
use super::master::{load_master_response_unlocked, load_master_task_unlocked};
use super::store::{
    ensure_bootstrap, load_v2_index_unlocked, persist_master, with_write_lock, LoadOutcome,
};
use super::types::{
    CategoriesFile, Category, TodoTasksIndex, DEFAULT_CATEGORY_ID, DEFAULT_CATEGORY_NAME,
};

pub(super) fn default_categories_file() -> CategoriesFile {
    CategoriesFile {
        version: 1,
        categories: vec![Category {
            id: DEFAULT_CATEGORY_ID.to_string(),
            name: DEFAULT_CATEGORY_NAME.to_string(),
            is_default: true,
        }],
    }
}

/// Load todo category registry via `paths::todo_tasks_categories_path`.
/// Missing file → in-memory built-in default「待分类」(does not require write).
pub fn load_categories() -> Result<CategoriesFile, String> {
    let path = paths::todo_tasks_categories_path().map_err(|e| format!("{e:?}"))?;
    if !path.is_file() {
        return Ok(default_categories_file());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Ensure built-in default category exists; create registry file when missing.
pub fn ensure_default_category() -> Result<CategoriesFile, String> {
    with_write_lock(|| ensure_default_category_unlocked())
}

pub(super) fn ensure_default_category_unlocked() -> Result<CategoriesFile, String> {
    let path = paths::todo_tasks_categories_path().map_err(|e| format!("{e:?}"))?;
    if !path.is_file() {
        let cats = default_categories_file();
        save_categories_unlocked(&cats)?;
        return Ok(cats);
    }
    let mut cats = load_categories()?;
    if !cats.categories.iter().any(|c| c.id == DEFAULT_CATEGORY_ID) {
        cats.categories.insert(
            0,
            Category {
                id: DEFAULT_CATEGORY_ID.to_string(),
                name: DEFAULT_CATEGORY_NAME.to_string(),
                is_default: true,
            },
        );
        save_categories_unlocked(&cats)?;
    }
    Ok(cats)
}

pub(super) fn save_categories_unlocked(cats: &CategoriesFile) -> Result<(), String> {
    let path = paths::todo_tasks_categories_path().map_err(|e| format!("{e:?}"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let value = serde_json::to_value(cats).map_err(|e| e.to_string())?;
    atomic_json::write_json(&path, &value)
}

pub(super) fn category_id_known(cats: &CategoriesFile, id: &str) -> bool {
    cats.categories.iter().any(|c| c.id == id)
}

pub(super) fn count_members_with_category(index: &TodoTasksIndex, category_id: &str) -> usize {
    index
        .tasks
        .values()
        .filter(|e| e.category_id == category_id)
        .count()
}

pub(super) fn resolve_create_category_id(
    category_id: Option<&str>,
    cats: &CategoriesFile,
) -> Result<String, TodoError> {
    match category_id.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(DEFAULT_CATEGORY_ID.to_string()),
        Some(id) if category_id_known(cats, id) => Ok(id.to_string()),
        Some(_) => Err(TodoError::bad_request("Invalid category_id")),
    }
}

/// List todo category registry (injects built-in default when file missing).
pub fn list_todo_categories() -> Result<Value, TodoError> {
    match load_categories() {
        Ok(cats) => {
            let categories = serde_json::to_value(&cats.categories).unwrap_or_else(|_| json!([]));
            Ok(json!({ "categories": categories }))
        }
        Err(e) => Err(TodoError::internal(e)),
    }
}

/// Create a non-default todo category; returns minted id.
pub fn create_todo_category(name: &str) -> Result<Value, TodoError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(TodoError::with_status(400, "Missing name"));
    }
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }
        let mut cats = match ensure_default_category_unlocked() {
            Ok(c) => c,
            Err(e) => return Err(TodoError::internal(e)),
        };
        let id = format!("cat{}", random_hex12());
        let category = Category {
            id: id.clone(),
            name: name.to_string(),
            is_default: false,
        };
        cats.categories.push(category.clone());
        if let Err(e) = save_categories_unlocked(&cats) {
            return Err(TodoError::internal(e));
        }
        Ok(json!({
            "category_id": id,
            "category": category
        }))
    })
}

/// Delete a category. Default and non-empty categories are rejected (no reassignment).
pub fn delete_todo_category(category_id: &str) -> Result<Value, TodoError> {
    let category_id = category_id.trim();
    if category_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing category_id"));
    }
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }
        let mut cats = match ensure_default_category_unlocked() {
            Ok(c) => c,
            Err(e) => return Err(TodoError::internal(e)),
        };
        let Some(entry) = cats.categories.iter().find(|c| c.id == category_id) else {
            return Err(TodoError::with_status(404, "Category not found"));
        };
        if entry.is_default || category_id == DEFAULT_CATEGORY_ID {
            return Err(TodoError::with_status(400, "Cannot delete default category"));
        }

        let index = match load_v2_index_unlocked() {
            Ok(i) => i,
            Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => TodoTasksIndex::default(),
        };
        if count_members_with_category(&index, category_id) > 0 {
            return Err(TodoError::with_status(400, "Category not empty"));
        }

        cats.categories.retain(|c| c.id != category_id);
        if let Err(e) = save_categories_unlocked(&cats) {
            return Err(TodoError::internal(e));
        }
        Ok(json!({ "ok": true }))
    })
}

/// Set a single todo's `category_id`. Unknown ids are rejected (no fallback).
pub fn set_master_category(master_task_id: &str, category_id: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    let category_id = category_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    if category_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing category_id"));
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }
        let cats = match ensure_default_category_unlocked() {
            Ok(c) => c,
            Err(e) => return Err(TodoError::internal(e)),
        };
        if !category_id_known(&cats, category_id) {
            return Err(TodoError::with_status(400, "Invalid category_id"));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };
        master.category_id = category_id.to_string();
        match persist_master(&master) {
            Ok(()) => match load_master_response_unlocked(master_task_id) {
                Ok(response) => Ok(json!({ "task": response })),
                Err(err) => Err(err),
            },
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub(super) fn effective_category_id(stored: &str, categories: &CategoriesFile) -> String {
    if stored.is_empty() {
        return DEFAULT_CATEGORY_ID.to_string();
    }
    if categories.categories.iter().any(|c| c.id == stored) {
        return stored.to_string();
    }
    DEFAULT_CATEGORY_ID.to_string()
}
