//! Plan attachments: stage, add, list, read, save, delete.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_hex12;
use crate::services::source_path_allow::{self, MAX_ATTACHMENT_SOURCE_BYTES};

use super::error::{bootstrap_error, corrupt_storage_error, TodoError};
use super::store::{
    ensure_bootstrap, load_v2_for_read, load_v2_index_unlocked, with_write_lock, LoadOutcome,
};
use super::types::{AttachmentEntry, AttachmentsFile, TodoTasksIndex};

#[cfg(test)]
use super::test_hooks::{
    test_take_fail_add_attachment_manifest, test_take_fail_delete_attachment_file,
};

/// Resolve an absolute source path under the attachment allow-list.
/// Returns `(canonical_path, normalized_basename)`.
pub(super) fn resolve_allowed_source_file(source_path: &str) -> Result<(PathBuf, String), TodoError> {
    let canonical = match source_path_allow::resolve_allowed_source_path(
        source_path,
        MAX_ATTACHMENT_SOURCE_BYTES,
    ) {
        Ok(p) => p,
        Err(mut err) => {
            if err.get("error").and_then(|v| v.as_str()) == Some("Source file too large") {
                err["error"] = json!("Attachment too large");
            }
            let status = err.get("_status").and_then(|v| v.as_u64()).unwrap_or(400) as u16;
            let message = err
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Invalid source path")
                .to_string();
            return Err(TodoError::with_status(status, message));
        }
    };
    let basename = canonical
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let normalized = match normalize_attachment_file_name(basename) {
        Ok(name) => name,
        Err(msg) => return Err(TodoError::bad_request(msg)),
    };
    Ok((canonical, normalized))
}

/// UI helper: write markdown into `cache_dir/todo_attachment_stage/<id>/<name>` and
/// return absolute `source_path` (not exposed via MCP / Sidecar).
pub fn stage_attachment_source(preferred_name: &str, content: &str) -> Result<Value, TodoError> {
    let basename = match normalize_attachment_file_name(preferred_name) {
        Ok(name) => name,
        Err(msg) => return Err(TodoError::bad_request(msg)),
    };
    if (content.len() as u64) > MAX_ATTACHMENT_SOURCE_BYTES {
        return Err(TodoError::with_status(413, "Attachment too large"));
    }
    let cache = match paths::cache_dir() {
        Ok(p) => p,
        Err(e) => return Err(TodoError::internal(format!("{e:?}"))),
    };
    let stage_dir = cache
        .join("todo_attachment_stage")
        .join(format!("s_{}", random_hex12()));
    if let Err(e) = fs::create_dir_all(&stage_dir) {
        return Err(TodoError::internal(e.to_string()));
    }
    let dest = stage_dir.join(&basename);
    if let Err(e) = fs::write(&dest, content) {
        return Err(TodoError::internal(e.to_string()));
    }
    let abs = dest.canonicalize().unwrap_or(dest);
    Ok(json!({
        "source_path": abs.to_string_lossy()
    }))
}

pub(super) fn normalize_attachment_file_name(file_name: &str) -> Result<String, &'static str> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("Invalid file name");
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("Invalid file name");
    }
    let path = Path::new(trimmed);
    if path.components().count() != 1 {
        return Err("Invalid file name");
    }
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return Err("Invalid file name");
    };
    if name != trimmed {
        return Err("Invalid file name");
    }
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Err("Only .md attachments are supported");
    };
    if !ext.eq_ignore_ascii_case("md") {
        return Err("Only .md attachments are supported");
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem.is_empty() {
        return Err("Invalid file name");
    }
    Ok(name.to_string())
}

pub(super) fn attachment_name_taken(dir: &Path, manifest: &AttachmentsFile, name: &str) -> bool {
    dir.join(name).exists() || manifest.attachments.iter().any(|e| e.file_name == name)
}

pub(super) fn resolve_unique_attachment_name(
    attachments_dir: &Path,
    manifest: &AttachmentsFile,
    basename: &str,
) -> String {
    if !attachment_name_taken(attachments_dir, manifest, basename) {
        return basename.to_string();
    }
    let path = Path::new(basename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(basename);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
    let mut n = 1u32;
    loop {
        let candidate = format!("{stem}-{n}.{ext}");
        if !attachment_name_taken(attachments_dir, manifest, &candidate) {
            return candidate;
        }
        n += 1;
    }
}

pub(super) fn load_attachments_file_unlocked(path: &Path) -> Result<AttachmentsFile, String> {
    if !path.is_file() {
        return Ok(AttachmentsFile {
            attachments: vec![],
        });
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub(super) fn persist_attachments_manifest(path: &Path, file: &AttachmentsFile) -> Result<(), String> {
    let value = serde_json::to_value(file).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

pub(super) fn write_attachments_file(path: &Path, file: &AttachmentsFile) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_add_attachment_manifest() {
        return Err("injected attachments.json write failure".to_string());
    }
    persist_attachments_manifest(path, file)
}

pub(super) fn restore_attachments_manifest(path: &Path, previous: &AttachmentsFile) -> Result<(), String> {
    if previous.attachments.is_empty() {
        if path.is_file() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    persist_attachments_manifest(path, previous)
}

pub(super) fn remove_attachment_file(path: &Path) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_delete_attachment_file() {
        return Err("injected attachment file delete failure".to_string());
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Copy a `.md` attachment into the plan task directory and append `attachments.json`.
/// On any post-copy failure, deletes the new file and restores the prior manifest.
pub fn add_attachment(master_task_id: &str, source_path: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }

    let (canonical_source, basename) = match resolve_allowed_source_file(source_path) {
        Ok(v) => v,
        Err(err) => return Err(err),
    };

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

        let task_dir = match paths::todo_tasks_task_dir(master_task_id) {
            Ok(p) => p,
            Err(e) => return Err(TodoError::internal(format!("{e:?}"))),
        };
        let attachments_dir = task_dir.join("attachments");
        let manifest_path = task_dir.join("attachments.json");

        let previous = match load_attachments_file_unlocked(&manifest_path) {
            Ok(f) => f,
            Err(e) => return Err(TodoError::internal(e)),
        };

        let final_name =
            resolve_unique_attachment_name(&attachments_dir, &previous, &basename);
        let dest = attachments_dir.join(&final_name);

        if let Err(e) = fs::create_dir_all(&attachments_dir) {
            return Err(TodoError::internal(e.to_string()));
        }
        if let Err(e) = fs::copy(&canonical_source, &dest) {
            return Err(TodoError::internal(e.to_string()));
        }

        let entry = AttachmentEntry {
            file_name: final_name,
            original_file_name: basename,
            added_at: Utc::now().to_rfc3339(),
        };
        let mut updated = previous.clone();
        updated.attachments.push(entry);

        if let Err(e) = write_attachments_file(&manifest_path, &updated) {
            let _ = fs::remove_file(&dest);
            let _ = restore_attachments_manifest(&manifest_path, &previous);
            return Err(TodoError::internal(e));
        }

        let stored = updated.attachments.last().expect("just pushed");
        Ok(json!({
            "file_name": stored.file_name,
            "original_file_name": stored.original_file_name,
            "added_at": stored.added_at
        }))
    })
}

pub(super) fn attachment_in_manifest(manifest: &AttachmentsFile, file_name: &str) -> bool {
    manifest.attachments.iter().any(|e| e.file_name == file_name)
}

/// Task dir + `attachments.json` for an existing plan. Missing manifest → empty list.
pub(super) fn load_plan_attachments_manifest(
    master_task_id: &str,
) -> Result<(std::path::PathBuf, AttachmentsFile), TodoError> {
    let task_dir = paths::todo_tasks_task_dir(master_task_id)
        .map_err(|e| TodoError::internal(format!("{e:?}")))?;
    let manifest = load_attachments_file_unlocked(&task_dir.join("attachments.json"))
        .map_err(|e| TodoError::internal(e))?;
    Ok((task_dir, manifest))
}

/// List attachments for a plan from `attachments.json` (SSOT; not a directory scan).
/// Missing manifest → empty collection. Unknown/unreadable plan → error.
pub fn list_attachments(master_task_id: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }

    match load_v2_for_read() {
        Ok(index) => {
            if !index.tasks.contains_key(master_task_id) {
                return Err(TodoError::with_status(404, "Task not found"));
            }
            match load_plan_attachments_manifest(master_task_id) {
                Ok((task_dir, file)) => {
                    let dir = task_dir.join("attachments");
                    let attachments: Vec<Value> = file
                        .attachments
                        .iter()
                        .map(|entry| {
                            json!({
                                "file_name": entry.file_name,
                                "original_file_name": entry.original_file_name,
                                "added_at": entry.added_at,
                                "path": dir.join(&entry.file_name).to_string_lossy(),
                            })
                        })
                        .collect();
                    Ok(json!({ "attachments": attachments }))
                }
                Err(err) => Err(err),
            }
        }
        Err(err) => Err(err),
    }
}

/// Read on-disk content of a manifest-listed attachment.
pub fn read_attachment(master_task_id: &str, file_name: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return Err(TodoError::with_status(400, "Invalid file name"));
    }

    match load_v2_for_read() {
        Ok(index) => {
            if !index.tasks.contains_key(master_task_id) {
                return Err(TodoError::with_status(404, "Task not found"));
            }
            let (task_dir, manifest) = match load_plan_attachments_manifest(master_task_id) {
                Ok(ctx) => ctx,
                Err(err) => return Err(err),
            };
            if !attachment_in_manifest(&manifest, file_name) {
                return Err(TodoError::with_status(404, "Attachment not found"));
            }
            let path = task_dir.join("attachments").join(file_name);
            match fs::read_to_string(&path) {
                Ok(content) => Ok(json!({
                    "file_name": file_name,
                    "content": content
                })),
                Err(e) => Err(TodoError::internal(e.to_string())),
            }
        }
        Err(err) => Err(err),
    }
}

/// Overwrite on-disk content of a manifest-listed attachment. Does not modify `todo.md`.
pub fn save_attachment(master_task_id: &str, file_name: &str, source_path: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return Err(TodoError::with_status(400, "Invalid file name"));
    }

    let (canonical_source, _) = match resolve_allowed_source_file(source_path) {
        Ok(v) => v,
        Err(err) => return Err(err),
    };

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

        let (task_dir, manifest) = match load_plan_attachments_manifest(master_task_id) {
            Ok(ctx) => ctx,
            Err(err) => return Err(err),
        };
        if !attachment_in_manifest(&manifest, file_name) {
            return Err(TodoError::with_status(404, "Attachment not found"));
        }

        let path = task_dir.join("attachments").join(file_name);
        match fs::copy(&canonical_source, &path) {
            Ok(_) => Ok(json!({ "ok": true })),
            Err(e) => Err(TodoError::internal(e.to_string())),
        }
    })
}

/// Remove a manifest-listed attachment entry and its on-disk file together.
/// On file-delete failure, restores the prior manifest (no half-success).
pub fn delete_attachment(master_task_id: &str, file_name: &str) -> Result<Value, TodoError> {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return Err(TodoError::with_status(400, "Missing id"));
    }
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return Err(TodoError::with_status(400, "Invalid file name"));
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

        let (task_dir, previous) = match load_plan_attachments_manifest(master_task_id) {
            Ok(ctx) => ctx,
            Err(err) => return Err(err),
        };
        if !attachment_in_manifest(&previous, file_name) {
            return Err(TodoError::with_status(404, "Attachment not found"));
        }

        let manifest_path = task_dir.join("attachments.json");
        let path = task_dir.join("attachments").join(file_name);

        let mut updated = previous.clone();
        updated.attachments.retain(|e| e.file_name != file_name);

        // Manifest first, then file — on file failure restore prior manifest (IV-7).
        if let Err(e) = persist_attachments_manifest(&manifest_path, &updated) {
            return Err(TodoError::internal(e));
        }
        if let Err(e) = remove_attachment_file(&path) {
            let _ = restore_attachments_manifest(&manifest_path, &previous);
            return Err(TodoError::internal(e));
        }
        Ok(json!({ "ok": true }))
    })
}
