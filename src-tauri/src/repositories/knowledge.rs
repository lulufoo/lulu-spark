use std::path::{Path, PathBuf};

/// Last path segment of a knowledge directory key.
/// Accepts a folder name (`topic`) or a leftover `owner/repo` registry key.
pub fn kb_dir_name(repo: &str) -> Result<&str, String> {
    let repo = repo.trim();
    if repo.is_empty() || repo.contains("..") || repo.contains('\\') || repo.contains('\0') {
        return Err("invalid directory name".into());
    }
    let name = repo.split('/').next_back().unwrap_or("");
    if name.is_empty() || name == "." || name == ".." {
        return Err("invalid directory name".into());
    }
    Ok(name)
}

fn folder_ok(folder: &str) -> Result<&str, String> {
    let folder = folder.trim();
    if folder.is_empty()
        || folder == "."
        || folder == ".."
        || folder.contains("..")
        || folder.contains('/')
        || folder.contains('\\')
        || folder.contains('\0')
    {
        return Err("invalid directory name".into());
    }
    Ok(folder)
}

/// Disk path: `{knowledge}/{folder}/{rel}`. `folder` is the topic folder, not the table id.
pub fn knowledge_file_path(kb_root: &Path, folder: &str, rel: &str) -> Result<PathBuf, String> {
    join(kb_root, folder, rel, false)
}

/// Topic folder, or a subdirectory under it. `rel` may be empty for the topic root.
pub fn knowledge_dir_path(kb_root: &Path, folder: &str, rel: &str) -> Result<PathBuf, String> {
    let target = join(kb_root, folder, rel, true)?;
    if !target.is_dir() {
        return Err("invalid path".into());
    }
    Ok(target)
}

fn join(kb_root: &Path, folder: &str, rel: &str, allow_empty_rel: bool) -> Result<PathBuf, String> {
    let folder = folder_ok(folder)?;
    let rel = rel.trim();
    if rel.contains("..") {
        return Err("invalid path".into());
    }
    if !allow_empty_rel && rel.is_empty() {
        return Err("invalid path".into());
    }
    let kb_canon = kb_root
        .canonicalize()
        .map_err(|e| format!("kb root: {e}"))?;
    let topic = kb_canon.join(folder);
    if !topic.is_dir() {
        return Err(format!("directory not found: {folder}"));
    }
    if rel.is_empty() {
        return Ok(topic);
    }
    Ok(topic.join(rel))
}

#[cfg(test)]
#[path = "../unit-tests/repositories/knowledge.rs"]
mod tests;
