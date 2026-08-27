//! Per-task todo.md read / write.

use serde_json::{json, Value};

use crate::config::paths;

use super::error::{bootstrap_error, corrupt_storage_error, TodoError};
use super::store::{
    ensure_bootstrap, load_v2_for_read, load_v2_index_unlocked, read_todo_md_or_empty,
    with_write_lock, write_plan_md_atomic, LoadOutcome,
};
use super::types::TodoTasksIndex;

pub fn read_todo_md(master_task_id: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }

    match load_v2_for_read() {
        Ok(index) => {
            if !index.tasks.contains_key(master_task_id) {
                return Err(TodoError::with_status(404, "Task not found"));
            }
            match read_todo_md_or_empty(master_task_id) {
                Ok(todo_md) => Ok(json!({ "todo_md": todo_md })),
                Err(e) => Err(TodoError::internal(e)),
            }
        }
        Err(err) => Err(err),
    }
}

pub fn update_todo_md(master_task_id: &str, plan_md: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }

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

        let plan_md_path = match paths::todo_tasks_plan_md_path(master_task_id) {
            Ok(p) => p,
            Err(e) => return Err(TodoError::internal(format!("{e:?}"))),
        };

        match write_plan_md_atomic(&plan_md_path, plan_md) {
            Ok(()) => Ok(json!({ "ok": true })),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}
