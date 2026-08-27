//! Plan process notes (comments.json).

use std::fs;
use std::path::Path;

use chrono::Utc;
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_hex12;

use super::error::{bootstrap_error, corrupt_storage_error, TodoError};
use super::store::{
    ensure_bootstrap, load_v2_for_read, load_v2_index_unlocked, with_write_lock, LoadOutcome,
};
use super::types::{CommentEntry, CommentsFile, TodoTasksIndex};

pub(crate) fn mint_comment_id() -> String {
    format!("cmt_{}", random_hex12())
}

pub(crate) fn now_comment_created_at() -> String {
    Utc::now().to_rfc3339()
}

pub(crate) fn validate_comment_body(body: &str) -> Result<(), String> {
    if body.trim().is_empty() {
        return Err("Comment body must not be empty".to_string());
    }
    Ok(())
}

pub(crate) fn load_comments_file_unlocked(path: &Path) -> Result<CommentsFile, String> {
    if !path.is_file() {
        return Ok(CommentsFile {
            comments: vec![],
        });
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub(crate) fn save_comments_file_unlocked(path: &Path, file: &CommentsFile) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let value = serde_json::to_value(file).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

/// Task dir + `comments.json` for an existing plan. Missing file → empty list.
pub(super) fn load_plan_comments(
    master_task_id: &str,
) -> Result<(std::path::PathBuf, CommentsFile), TodoError> {
    let task_dir = paths::todo_tasks_task_dir(master_task_id)
        .map_err(|e| TodoError::internal(format!("{e:?}")))?;
    let file = load_comments_file_unlocked(&task_dir.join("comments.json"))
        .map_err(|e| TodoError::internal(e))?;
    Ok((task_dir, file))
}

pub(super) fn comments_sorted_by_created_at(mut file: CommentsFile) -> CommentsFile {
    file.comments
        .sort_by(|a, b| a.created_at.cmp(&b.created_at));
    file
}

/// List comments for a plan from `comments.json`. Missing file → empty collection.
/// Entries are returned ordered by `created_at` ascending.
pub fn list_comments(master_task_id: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }

    match load_v2_for_read() {
        Ok(index) => {
            if !index.tasks.contains_key(master_task_id) {
                return Err(TodoError::with_status(404, "Task not found"));
            }
            match load_plan_comments(master_task_id) {
                Ok((_, file)) => {
                    let file = comments_sorted_by_created_at(file);
                    let comments =
                        serde_json::to_value(&file.comments).unwrap_or_else(|_| json!([]));
                    Ok(json!({ "comments": comments }))
                }
                Err(err) => Err(err),
            }
        }
        Err(err) => Err(err),
    }
}

/// Append a comment with minted `id` / `created_at`. Rejects empty/whitespace body.
/// Must not refuse based on existing comment count.
pub fn add_comment(master_task_id: &str, body: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    if let Err(e) = validate_comment_body(body) {
        return Err(TodoError::bad_request(e));
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

        let (task_dir, previous) = match load_plan_comments(master_task_id) {
            Ok(ctx) => ctx,
            Err(err) => return Err(err),
        };
        let path = task_dir.join("comments.json");

        let entry = CommentEntry {
            id: mint_comment_id(),
            body: body.to_string(),
            created_at: now_comment_created_at(),
        };
        let mut updated = previous;
        updated.comments.push(entry.clone());

        if let Err(e) = save_comments_file_unlocked(&path, &updated) {
            return Err(TodoError::internal(e));
        }

        Ok(json!({
            "id": entry.id,
            "body": entry.body,
            "created_at": entry.created_at
        }))
    })
}

/// Update only `body` for an existing comment id. Preserves `id` and `created_at`.
pub fn update_comment(master_task_id: &str, comment_id: &str, body: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    let comment_id = comment_id.trim();
    if comment_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing comment id"));
    }
    if let Err(e) = validate_comment_body(body) {
        return Err(TodoError::bad_request(e));
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

        let (task_dir, mut file) = match load_plan_comments(master_task_id) {
            Ok(ctx) => ctx,
            Err(err) => return Err(err),
        };
        let path = task_dir.join("comments.json");

        let Some(pos) = file.comments.iter().position(|c| c.id == comment_id) else {
            return Err(TodoError::with_status(404, "Comment not found"));
        };
        file.comments[pos].body = body.to_string();

        if let Err(e) = save_comments_file_unlocked(&path, &file) {
            return Err(TodoError::internal(e));
        }

        let updated = &file.comments[pos];
        Ok(json!({
            "id": updated.id,
            "body": updated.body,
            "created_at": updated.created_at
        }))
    })
}

/// Hard-delete a single comment by id.
pub fn delete_comment(master_task_id: &str, comment_id: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    let comment_id = comment_id.trim();
    if comment_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing comment id"));
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

        let (task_dir, mut file) = match load_plan_comments(master_task_id) {
            Ok(ctx) => ctx,
            Err(err) => return Err(err),
        };
        let path = task_dir.join("comments.json");

        let before = file.comments.len();
        file.comments.retain(|c| c.id != comment_id);
        if file.comments.len() == before {
            return Err(TodoError::with_status(404, "Comment not found"));
        }

        if let Err(e) = save_comments_file_unlocked(&path, &file) {
            return Err(TodoError::internal(e));
        }

        Ok(json!({ "ok": true }))
    })
}
