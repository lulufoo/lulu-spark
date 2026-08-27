//! Index, locks, and atomic task-dir writes.

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use crate::config::paths;
use crate::repositories::atomic_json;

use super::error::{bootstrap_error, corrupt_storage_error, TodoError};
use super::migrate::migrate_implicit_subs;
use super::types::{
    index_entry_task_dir, IndexEntry, MasterTask, SubTasksFile, TodoTasksIndex, DEFAULT_CATEGORY_ID,
};

#[cfg(test)]
use super::test_hooks::{
    test_take_fail_batch_index, test_take_fail_batch_plan_md, test_take_fail_batch_sub_tasks,
};

static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub(super) fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().expect("todo_task write lock");
    f()
}

pub(super) fn delete_v1_if_present(path: &std::path::Path) -> Result<bool, String> {
    if path.is_file() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

pub(super) fn init_v2_index() -> Result<(), String> {
    let plan_dir = paths::todo_tasks_dir().map_err(|e| format!("{e:?}"))?;
    fs::create_dir_all(plan_dir.join("tasks")).map_err(|e| e.to_string())?;
    let index_path = paths::todo_tasks_index_path().map_err(|e| format!("{e:?}"))?;

    if index_path.is_file() {
        if let Ok(text) = fs::read_to_string(&index_path) {
            if let Ok(index) = serde_json::from_str::<TodoTasksIndex>(&text) {
                if index.version == 2 {
                    return Ok(());
                }
            }
        }
    }

    let index = TodoTasksIndex::default();
    write_todo_tasks_index(&index_path, &index)
}

pub(super) fn write_todo_tasks_index(path: &std::path::Path, index: &TodoTasksIndex) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub(super) fn ensure_bootstrap() -> Result<(), String> {
    let wb_v1 = paths::todo_tasks_path().map_err(|e| format!("{e:?}"))?;
    let cache_v1 = paths::cache_todo_tasks_v1_path().map_err(|e| format!("{e:?}"))?;

    let _deleted_wb = delete_v1_if_present(&wb_v1)?;
    let _deleted_cache = delete_v1_if_present(&cache_v1)?;

    init_v2_index()?;
    migrate_implicit_subs();
    Ok(())
}

pub(super) fn snapshot_index() -> Result<TodoTasksIndex, String> {
    let index_path = paths::todo_tasks_index_path().map_err(|e| format!("{e:?}"))?;
    if !index_path.is_file() {
        return Ok(TodoTasksIndex::default());
    }
    let text = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub(super) fn write_plan_md_atomic(path: &Path, content: &str) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_batch_plan_md() {
        return Err("injected plan.md write failure".to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub(super) fn write_sub_tasks_json(path: &Path, sub_tasks: &SubTasksFile) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_batch_sub_tasks() {
        return Err("injected sub_tasks.json write failure".to_string());
    }
    let value = serde_json::to_value(sub_tasks).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

pub(super) fn write_index_snapshot(path: &Path, index: &TodoTasksIndex) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_batch_index() {
        return Err("injected index.json write failure".to_string());
    }
    if index.tasks.is_empty() {
        return write_todo_tasks_index(path, index);
    }
    let value = serde_json::to_value(index).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

pub(super) fn rollback_before_index(master_task_id: &str) -> Result<(), String> {
    let task_dir = paths::todo_tasks_task_dir(master_task_id).map_err(|e| format!("{e:?}"))?;
    if task_dir.exists() {
        // Whole task_dir cascade — includes attachments/, attachments.json, comments.json.
        fs::remove_dir_all(&task_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn rollback_index_failure(snapshot: &TodoTasksIndex) -> Result<(), String> {
    let index_path = paths::todo_tasks_index_path().map_err(|e| format!("{e:?}"))?;
    write_index_snapshot(&index_path, snapshot)
}

/// I-2 atomic batch: sub_tasks.json → todo.md → index entry. Caller must hold `WRITE_LOCK`.
pub(super) fn write_task_batch(
    master_task_id: &str,
    index_entry: &IndexEntry,
    sub_tasks: &SubTasksFile,
    plan_md: &str,
) -> Result<(), String> {
    let snapshot = snapshot_index()?;
    let task_dir = paths::todo_tasks_task_dir(master_task_id).map_err(|e| format!("{e:?}"))?;
    fs::create_dir_all(&task_dir).map_err(|e| e.to_string())?;

    let sub_tasks_path = paths::todo_tasks_sub_tasks_path(master_task_id).map_err(|e| format!("{e:?}"))?;
    let plan_md_path = paths::todo_tasks_plan_md_path(master_task_id).map_err(|e| format!("{e:?}"))?;
    let index_path = paths::todo_tasks_index_path().map_err(|e| format!("{e:?}"))?;

    if let Err(e) = write_sub_tasks_json(&sub_tasks_path, sub_tasks) {
        let _ = rollback_before_index(master_task_id);
        return Err(e);
    }

    if let Err(e) = write_plan_md_atomic(&plan_md_path, plan_md) {
        let _ = rollback_before_index(master_task_id);
        return Err(e);
    }

    let mut index = snapshot.clone();
    index
        .tasks
        .insert(master_task_id.to_string(), index_entry.clone());

    if let Err(e) = write_index_snapshot(&index_path, &index) {
        let _ = rollback_index_failure(&snapshot);
        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
pub fn test_run_write_task_batch(
    master_task_id: &str,
    index_entry: &IndexEntry,
    sub_tasks: &SubTasksFile,
    plan_md: &str,
) -> Result<(), String> {
    with_write_lock(|| {
        ensure_bootstrap()?;
        write_task_batch(master_task_id, index_entry, sub_tasks, plan_md)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LoadOutcome {
    Ok,
    Missing,
    Corrupt,
}

pub(super) fn load_v2_index_unlocked() -> Result<TodoTasksIndex, LoadOutcome> {
    let index_path = match paths::todo_tasks_index_path() {
        Ok(p) => p,
        Err(_) => return Ok(TodoTasksIndex::default()),
    };
    if !index_path.is_file() {
        return Ok(TodoTasksIndex::default());
    }
    let text = match fs::read_to_string(&index_path) {
        Ok(t) => t,
        Err(_) => return Err(LoadOutcome::Corrupt),
    };
    match serde_json::from_str::<TodoTasksIndex>(&text) {
        Ok(index) if index.version == 2 => Ok(index),
        _ => Err(LoadOutcome::Corrupt),
    }
}

pub(super) fn load_sub_tasks_file(master_id: &str) -> Result<SubTasksFile, LoadOutcome> {
    let path = match paths::todo_tasks_sub_tasks_path(master_id) {
        Ok(p) => p,
        Err(_) => return Err(LoadOutcome::Corrupt),
    };
    if !path.is_file() {
        return Err(LoadOutcome::Corrupt);
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Err(LoadOutcome::Corrupt),
    };
    serde_json::from_str(&text).map_err(|_| LoadOutcome::Corrupt)
}


pub(super) fn load_v2_for_read() -> Result<TodoTasksIndex, TodoError> {
    if let Err(e) = ensure_bootstrap() {
        return Err(bootstrap_error(e));
    }
    match load_v2_index_unlocked() {
        Ok(index) => Ok(index),
        Err(LoadOutcome::Corrupt) => Err(corrupt_storage_error()),
        Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => Ok(TodoTasksIndex::default()),
    }
}

pub(super) fn read_todo_md_or_empty(master_id: &str) -> Result<String, String> {
    let path = paths::todo_tasks_plan_md_path(master_id).map_err(|e| format!("{e:?}"))?;
    if !path.is_file() {
        return Ok(String::new());
    }
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

pub(super) fn persist_master(master: &MasterTask) -> Result<(), String> {
    let plan_md = read_todo_md_or_empty(&master.master_task_id)?;
    persist_master_with_plan_md(master, &plan_md)
}

pub(super) fn persist_master_with_plan_md(master: &MasterTask, plan_md: &str) -> Result<(), String> {
    let entry = IndexEntry {
        master_task_id: master.master_task_id.clone(),
        title: master.title.clone(),
        status: master.status.clone(),
        created_at: master.created_at.clone(),
        task_dir: index_entry_task_dir(&master.master_task_id),
        category_id: if master.category_id.is_empty() {
            DEFAULT_CATEGORY_ID.to_string()
        } else {
            master.category_id.clone()
        },
    };
    let subs = SubTasksFile {
        sub_tasks: master.sub_tasks.clone(),
    };
    write_task_batch(&master.master_task_id, &entry, &subs, plan_md)
}
