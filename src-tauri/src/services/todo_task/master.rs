//! Master task create / read / list / delete / title / status.

use chrono::Utc;
use serde_json::{json, Value};

use crate::config::paths;
use crate::services::id::random_hex12;

use super::categories::{
    default_categories_file, effective_category_id, ensure_default_category_unlocked, load_categories,
    resolve_create_category_id,
};
use super::error::{bootstrap_error, corrupt_storage_error, TodoError};
use super::migrate::plan_has_migration_error;
use super::store::{
    ensure_bootstrap, load_sub_tasks_file, load_v2_for_read, load_v2_index_unlocked,
    persist_master, persist_master_with_plan_md, read_todo_md_or_empty, rollback_before_index,
    with_write_lock, write_index_snapshot, LoadOutcome,
};
use super::types::{
    IndexEntry, MasterTask, MasterTaskStatus, SubTask, SubTaskStatus, SubTasksFile, TodoTasksIndex,
};

pub(super) fn assemble_master_task(entry: &IndexEntry, subs: &SubTasksFile) -> MasterTask {
    let categories = load_categories().unwrap_or_else(|_| default_categories_file());
    let category_id = effective_category_id(&entry.category_id, &categories);
    MasterTask {
        master_task_id: entry.master_task_id.clone(),
        title: entry.title.clone(),
        status: entry.status.clone(),
        created_at: entry.created_at.clone(),
        sub_tasks: subs.sub_tasks.clone(),
        category_id,
    }
}

pub(super) fn load_master_task_unlocked(master_id: &str) -> Result<MasterTask, TodoError> {
    let index = match load_v2_index_unlocked() {
        Ok(i) => i,
        Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
        Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => TodoTasksIndex::default(),
    };
    let entry = index
        .tasks
        .get(master_id)
        .ok_or_else(|| TodoError::not_found("Task not found"))?;
    let subs = match load_sub_tasks_file(master_id) {
        Ok(s) => s,
        Err(LoadOutcome::Corrupt) | Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => {
            return Err(corrupt_storage_error());
        }
    };
    Ok(assemble_master_task(entry, &subs))
}

pub(super) fn find_master_id_for_any_id(index: &TodoTasksIndex, id: &str) -> Result<Option<String>, TodoError> {
    if index.tasks.contains_key(id) {
        return Ok(Some(id.to_string()));
    }
    for master_id in index.tasks.keys() {
        let subs = match load_sub_tasks_file(master_id) {
            Ok(s) => s,
            Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => return Err(corrupt_storage_error()),
        };
        if subs.sub_tasks.iter().any(|s| s.sub_task_id == id) {
            return Ok(Some(master_id.clone()));
        }
    }
    Ok(None)
}

/// Count title units: each CJK ideograph = 1; each whitespace-delimited word token = 1.
/// Punctuation and other symbols do not count.
pub fn title_unit_count(title: &str) -> usize {
    let mut units = 0usize;
    let mut in_word = false;
    for c in title.chars() {
        if is_cjk_ideograph(c) {
            if in_word {
                units += 1;
                in_word = false;
            }
            units += 1;
        } else if c.is_whitespace() {
            if in_word {
                units += 1;
                in_word = false;
            }
        } else if c.is_ascii_alphanumeric()
            || matches!(c, '\u{00C0}'..='\u{024F}' | '\u{1E00}'..='\u{1EFF}')
        {
            in_word = true;
        } else if in_word {
            units += 1;
            in_word = false;
        }
    }
    if in_word {
        units += 1;
    }
    units
}

pub(super) fn is_cjk_ideograph(c: char) -> bool {
    matches!(
        c,
        '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
            | '\u{2CEB0}'..='\u{2EBEF}'
            | '\u{30000}'..='\u{3134F}'
    )
}

pub(super) const TITLE_UNIT_LIMIT: usize = 20;

pub(super) fn new_master_id() -> String {
    format!("task_{}", random_hex12())
}

pub(super) fn format_sub_id(master_id: &str, index: usize) -> String {
    format!("{master_id}_sub_{index:02}")
}

pub(super) fn master_to_value(master: &MasterTask) -> Value {
    serde_json::to_value(master).unwrap_or_else(|_| json!({}))
}

pub(super) fn master_to_response(master: &MasterTask, migration_error: bool) -> Value {
    let todo_md = read_todo_md_or_empty(&master.master_task_id).unwrap_or_default();
    let mut value = master_to_value(master);
    if let Value::Object(ref mut map) = value {
        map.insert("todo_md".to_string(), json!(todo_md));
        map.insert("migration_error".to_string(), json!(migration_error));
    }
    value
}

pub(super) fn load_master_response_unlocked(master_id: &str) -> Result<Value, TodoError> {
    let index = match load_v2_index_unlocked() {
        Ok(i) => i,
        Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
        Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => TodoTasksIndex::default(),
    };
    let entry = index
        .tasks
        .get(master_id)
        .ok_or_else(|| TodoError::not_found("Task not found"))?;
    let migration_error = plan_has_migration_error(master_id);
    if migration_error {
        let subs = match load_sub_tasks_file(master_id) {
            Ok(s) => s,
            Err(_) => SubTasksFile {
                sub_tasks: vec![],
            },
        };
        return Ok(master_to_response(
            &assemble_master_task(entry, &subs),
            true,
        ));
    }
    let subs = match load_sub_tasks_file(master_id) {
        Ok(s) => s,
        Err(LoadOutcome::Corrupt) | Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => {
            return Err(corrupt_storage_error());
        }
    };
    Ok(master_to_response(
        &assemble_master_task(entry, &subs),
        false,
    ))
}

pub fn create_master_with_subs(title: &str, sub_titles: Option<&[&str]>) -> Result<Value, TodoError> {
    create_master_with_subs_and_todo(title, sub_titles, "")
}

/// Create a master task. `plan_md` is written atomically with the task batch (empty → empty `todo.md`).
pub fn create_master_with_subs_and_todo(
    title: &str,
    sub_titles: Option<&[&str]>,
    plan_md: &str,
) -> Result<Value, TodoError> {
    create_master_with_category(title, sub_titles, plan_md, None)
}

/// Create a master task with optional `category_id`.
/// Omit/empty → default「待分类」; unknown id → reject (no fallback).
pub fn create_master_with_category(
    title: &str,
    sub_titles: Option<&[&str]>,
    plan_md: &str,
    category_id: Option<&str>,
) -> Result<Value, TodoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TodoError::with_status(400, "Missing title"));
    }
    if title_unit_count(title) > TITLE_UNIT_LIMIT {
        return Err(TodoError::with_status(400, "Title too long (max 20 Chinese characters or English words)"));
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        if let Err(LoadOutcome::Corrupt) = load_v2_index_unlocked() {
            return Err(corrupt_storage_error());
        }

        let cats = match ensure_default_category_unlocked() {
            Ok(c) => c,
            Err(e) => return Err(TodoError::internal(e)),
        };
        let resolved_category_id = match resolve_create_category_id(category_id, &cats) {
            Ok(id) => id,
            Err(err) => return Err(err),
        };

        let master_id = new_master_id();
        let created_at = Utc::now().to_rfc3339();
        let titles: Vec<String> = match sub_titles {
            Some(slice) if !slice.is_empty() => {
                for t in slice {
                    if t.trim().is_empty() {
                        return Err(TodoError::with_status(400, "Invalid sub_titles element"));
                    }
                }
                slice.iter().map(|t| t.trim().to_string()).collect()
            }
            _ => vec![],
        };

        let sub_tasks = if titles.is_empty() {
            vec![]
        } else {
            titles
                .iter()
                .enumerate()
                .map(|(i, t)| SubTask {
                    sub_task_id: format_sub_id(&master_id, i + 1),
                    title: Some(t.clone()),
                    content: None,
                    status: SubTaskStatus::Incomplete,
                    implicit: false,
                    linked_archive_ids: vec![],
                    completed_at: None,
                })
                .collect()
        };

        let master = MasterTask {
            master_task_id: master_id.clone(),
            title: title.to_string(),
            status: MasterTaskStatus::Incomplete,
            created_at,
            sub_tasks,
            category_id: resolved_category_id,
        };

        match persist_master_with_plan_md(&master, plan_md) {
            Ok(()) => {
                let mut response = json!({
                    "master_task_id": master_id,
                    "task": master_to_value(&master)
});
                if let Some(first_sub) = master.sub_tasks.first() {
                    response["sub_task_id"] = json!(first_sub.sub_task_id);
                }
                Ok(response)
            }
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub fn get_by_id(id: &str) -> Result<Value, TodoError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }

    match load_v2_for_read() {
        Ok(index) => match find_master_id_for_any_id(&index, id) {
            Ok(Some(master_id)) => match load_master_response_unlocked(&master_id) {
                Ok(response) => Ok(response),
                Err(err) => Err(err),
            },
            Ok(None) => Err(TodoError::with_status(404, "Task not found")),
            Err(err) => Err(err),
        },
        Err(err) => Err(err),
    }
}

pub fn list_all() -> Result<Value, TodoError> {
    match load_v2_for_read() {
        Ok(index) => {
            let mut masters: Vec<Value> = Vec::new();
            for master_id in index.tasks.keys() {
                match load_master_response_unlocked(master_id) {
                    Ok(response) => masters.push(response),
                    Err(err) => return Err(err),
                }
            }
            masters.sort_by(|a, b| {
                let a_ts = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                let b_ts = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                b_ts.cmp(a_ts)
            });
            Ok(Value::Array(masters))
        }
        Err(err) => Err(err),
    }
}

pub fn delete_master(master_task_id: &str) -> Result<Value, TodoError> {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let index = match load_v2_index_unlocked() {
            Ok(i) => i,
            Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => TodoTasksIndex::default(),
        };

        if !index.tasks.contains_key(master_task_id) {
            return Err(TodoError::with_status(404, "Task not found"));
        }

        if let Err(e) = rollback_before_index(master_task_id) {
            return Err(TodoError::internal(e));
        }

        let index_path = match paths::todo_tasks_index_path() {
            Ok(p) => p,
            Err(e) => return Err(TodoError::internal(format!("{e:?}"))),
        };

        let mut updated = index;
        updated.tasks.remove(master_task_id);
        match write_index_snapshot(&index_path, &updated) {
            Ok(()) => Ok(json!({ "ok": true })),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub fn update_master_title(master_task_id: &str, title: &str) -> Result<Value, TodoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TodoError::with_status(400, "Missing title"));
    }
    if title_unit_count(title) > TITLE_UNIT_LIMIT {
        return Err(TodoError::with_status(400, "Title too long (max 20 Chinese characters or English words)"));
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        master.title = title.to_string();
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "task": master_to_value(&updated)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

/// Update master `title` and/or body (`todo_md`) in one write.
///
/// - `title` / `todo_md`: `None` → leave unchanged; `Some` → apply.
/// - At least one of `title` / `todo_md` must be `Some`.
/// - `todo_md: Some("")` clears the body file.
/// - Success returns `{ "task": <get-shaped master with todo_md> }` (`_status` is L1-only).
pub fn update_master_fields(
    master_task_id: &str,
    title: Option<&str>,
    todo_md: Option<&str>,
) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    if title.is_none() && todo_md.is_none() {
        return Err(TodoError::with_status(400, "Missing title or todo_md"));
    }

    let normalized_title = match title {
        None => None,
        Some(t) => {
            let t = t.trim();
            if t.is_empty() {
                return Err(TodoError::with_status(400, "Missing title"));
            }
            if title_unit_count(t) > TITLE_UNIT_LIMIT {
                return Err(TodoError::with_status(400, "Title too long (max 20 Chinese characters or English words)"));
            }
            Some(t.to_string())
        }
    };

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        if let Some(ref t) = normalized_title {
            master.title = t.clone();
        }

        let existing_md;
        let plan_md_ref = match todo_md {
            Some(md) => md,
            None => {
                existing_md = match read_todo_md_or_empty(master_task_id) {
                    Ok(s) => s,
                    Err(e) => return Err(TodoError::internal(e)),
                };
                existing_md.as_str()
            }
        };

        match persist_master_with_plan_md(&master, plan_md_ref) {
            Ok(()) => match load_master_response_unlocked(master_task_id) {
                Ok(response) => Ok(json!({
                    "task": response
                })),
                Err(err) => Err(err),
            },
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub(super) fn parse_master_status_wire(status: &str) -> Option<MasterTaskStatus> {
    match status.trim() {
        "incomplete" => Some(MasterTaskStatus::Incomplete),
        "complete" => Some(MasterTaskStatus::Complete),
        "abandoned" => Some(MasterTaskStatus::Abandoned),
        _ => None,
    }
}

/// Host/UI-only explicit master status write (tri-state mutual transitions).
pub fn set_master_status(master_task_id: &str, status: &str) -> Result<Value, TodoError> {
    let Some(next) = parse_master_status_wire(status) else {
        return Err(TodoError::with_status(400, "Invalid status"));
    };

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        master.status = next;
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "task": master_to_value(&updated)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}
