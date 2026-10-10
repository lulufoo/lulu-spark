//! List and delete regular files under one Chat session scratch (SESSION_WORKSPACE_DIR).

use std::fs;
use std::path::Path;

use crate::services::path_fence::{require_absolute, stored_path, PathFence};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFile {
    pub path: String,
    pub title: String,
}

/// Regular files under `{scratch_parent}/{session_id}`. Empty when the directory
/// is missing. Skips hidden names and symlinks.
pub fn list_session_workspace_files(
    session_id: &str,
    fence: &PathFence,
) -> Result<Vec<WorkspaceFile>, String> {
    let Some(root) = fence.session_scratch_root(session_id)? else {
        return Ok(Vec::new());
    };
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let root = stored_path(root);
    let mut files = Vec::new();
    collect_regular_files(&root, &root, &mut files)?;
    files.sort_by(|a, b| a.title.cmp(&b.title));
    Ok(files)
}

fn collect_regular_files(
    root: &Path,
    dir: &Path,
    out: &mut Vec<WorkspaceFile>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for ent in entries {
        let ent = ent.map_err(|e| e.to_string())?;
        let name = ent.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let path = ent.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            collect_regular_files(root, &path, out)?;
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        let canon = stored_path(path);
        out.push(WorkspaceFile {
            path: canon.to_string_lossy().into_owned(),
            title: relative_title(root, &canon),
        });
    }
    Ok(())
}

/// Delete one regular file under this session scratch. Does not delete directories.
pub fn delete_session_workspace_file(
    session_id: &str,
    path: &str,
    fence: &PathFence,
) -> Result<Vec<WorkspaceFile>, String> {
    let Some(root) = fence.session_scratch_root(session_id)? else {
        return Err("path must be in SESSION_WORKSPACE_DIR".into());
    };
    let live = fence
        .clone()
        .with_session_scratch(session_id)
        .unwrap_or_else(|_| fence.clone());
    let abs = require_absolute(path)?;
    let meta = fs::symlink_metadata(&abs).map_err(|_| "path does not exist".to_string())?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err("path must be a regular file".into());
    }
    let canon = stored_path(abs);
    if !live.is_session_workspace(&canon) {
        return Err("path must be in SESSION_WORKSPACE_DIR".into());
    }
    let root = stored_path(root);
    if !canon.starts_with(&root) {
        return Err("path must be in SESSION_WORKSPACE_DIR".into());
    }
    fs::remove_file(&canon).map_err(|e| e.to_string())?;
    list_session_workspace_files(session_id, fence)
}

fn relative_title(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
#[path = "../unit-tests/services/chat_workspace.rs"]
mod tests;
