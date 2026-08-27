//! Sub-task create / update / complete / abandon / archive link.

use chrono::Utc;
use serde_json::{json, Value};

use super::error::{bootstrap_error, TodoError};
use super::master::{format_sub_id, load_master_task_unlocked, master_to_value};
use super::store::{ensure_bootstrap, persist_master, with_write_lock};
use super::types::{MasterTaskStatus, SubTask, SubTaskStatus};

#[cfg(test)]
use super::test_hooks::{test_take_fail_complete_sub, test_take_fail_link_archive};

pub(super) fn is_terminal_sub_status(status: &SubTaskStatus) -> bool {
    matches!(
        status,
        SubTaskStatus::Complete | SubTaskStatus::Abandoned
    )
}

pub fn add_sub(master_task_id: &str, title: &str, content: Option<&str>) -> Result<Value, TodoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TodoError::with_status(400, "Missing title"));
    }
    let content = content
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        let next_index = master.sub_tasks.len() + 1;
        let new_sub = SubTask {
            sub_task_id: format_sub_id(master_task_id, next_index),
            title: Some(title.to_string()),
            content,
            status: SubTaskStatus::Incomplete,
            implicit: false,
            linked_archive_ids: vec![],
            completed_at: None,
        };
        let new_sub_id = new_sub.sub_task_id.clone();
        master.sub_tasks.push(new_sub);

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "sub_task_id": new_sub_id,
                "task": master_to_value(&master)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub fn delete_sub(master_task_id: &str, sub_task_id: &str) -> Result<Value, TodoError> {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        let pos = match master
            .sub_tasks
            .iter()
            .position(|s| s.sub_task_id == sub_task_id)
        {
            Some(p) => p,
            None => return Err(TodoError::with_status(404, "Task not found")),
        };

        master.sub_tasks.remove(pos);

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "task": master_to_value(&master)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub fn complete_sub(master_task_id: &str, sub_task_id: &str) -> Result<Value, TodoError> {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        #[cfg(test)]
        if test_take_fail_complete_sub() {
            return Err(TodoError::with_status(500, "injected todo_task failure"));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return Err(TodoError::with_status(404, "Task not found"));
        };

        if is_terminal_sub_status(&sub.status) {
            return Err(TodoError::with_status(409, "Sub task is in terminal status"));
        }

        sub.status = SubTaskStatus::Complete;
        sub.completed_at = Some(Utc::now().to_rfc3339());
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "task": master_to_value(&updated)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub fn complete_todo(master_task_id: &str, sub_task_id: Option<&str>) -> Result<Value, TodoError> {
    match sub_task_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(sub_id) => complete_sub(master_task_id, sub_id),
        None => complete_master(master_task_id),
    }
}

pub(super)  fn complete_master(master_task_id: &str) -> Result<Value, TodoError> {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        match master.status {
            MasterTaskStatus::Complete => Ok(json!({
                "task": master_to_value(&master)
})),
            MasterTaskStatus::Abandoned => {
                Err(TodoError::with_status(409, "master_abandoned"))
            }
            MasterTaskStatus::Incomplete => {
                master.status = MasterTaskStatus::Complete;
                let updated = master.clone();
                match persist_master(&master) {
                    Ok(()) => Ok(json!({
                        "task": master_to_value(&updated)
                    })),
                    Err(e) => Err(TodoError::internal(e)),
                }
            }
        }
    })
}

pub fn update_sub_title(
    master_task_id: &str,
    sub_task_id: &str,
    title: &str,
    content: Option<&str>,
) -> Result<Value, TodoError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(TodoError::with_status(400, "Missing title"));
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return Err(TodoError::with_status(404, "Task not found"));
        };

        sub.title = Some(title.to_string());
        if let Some(c) = content {
            sub.content = if c.is_empty() {
                None
            } else {
                Some(c.to_string())
            };
        }
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "task": master_to_value(&updated)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub fn abandon_sub(master_task_id: &str, sub_task_id: &str) -> Result<Value, TodoError> {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return Err(TodoError::with_status(404, "Task not found"));
        };

        if is_terminal_sub_status(&sub.status) {
            return Err(TodoError::with_status(409, "Sub task is in terminal status"));
        }

        sub.status = SubTaskStatus::Abandoned;
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "task": master_to_value(&updated)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}

pub fn link_archive(master_task_id: &str, sub_task_id: &str, archive_id: &str) -> Result<Value, TodoError> {
    let archive_id = archive_id.trim();
    if archive_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing archive id"));
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return Err(bootstrap_error(e));
        }

        #[cfg(test)]
        if test_take_fail_link_archive() {
            return Err(TodoError::with_status(500, "injected link_archive failure"));
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return Err(err),
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return Err(TodoError::with_status(404, "Task not found"));
        };

        if !sub.linked_archive_ids.iter().any(|id| id == archive_id) {
            sub.linked_archive_ids.push(archive_id.to_string());
        }
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => Ok(json!({
                "task": master_to_value(&updated)
})),
            Err(e) => Err(TodoError::internal(e)),
        }
    })
}
