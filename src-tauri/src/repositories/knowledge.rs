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

/// Aligns with `server.py::_kb_safe_path`.
pub fn kb_safe_path(kb_root: &Path, repo: &str, rel_path: &str) -> Result<PathBuf, String> {
    let rel_path = rel_path.trim();
    if rel_path.is_empty() || rel_path.contains("..") {
        return Err("invalid path".into());
    }
    let repo_name = kb_dir_name(repo)?;
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return Err(format!("directory not found: {repo_name}"));
    }
    let kb_canon = kb_root
        .canonicalize()
        .map_err(|e| format!("kb root: {e}"))?;
    let mut target = local_dir
        .canonicalize()
        .unwrap_or_else(|_| local_dir.clone());
    for comp in Path::new(rel_path).components() {
        match comp {
            std::path::Component::Normal(s) => target.push(s),
            std::path::Component::CurDir => {}
            _ => return Err("invalid path".into()),
        }
    }
    let prefix = format!(
        "{}{}",
        kb_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    let target_str = target.to_string_lossy();
    if !target_str.starts_with(&prefix) && target != kb_canon {
        return Err("path traversal not allowed".into());
    }
    Ok(target)
}

/// List-only path resolver: allows `rel_path == ""` for directory root.
pub fn kb_list_dir(kb_root: &Path, repo: &str, rel_path: &str) -> Result<PathBuf, String> {
    let rel_path = rel_path.trim();
    if rel_path.contains("..") {
        return Err("invalid path".into());
    }
    let repo_name = kb_dir_name(repo)?;
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return Err(format!("directory not found: {repo_name}"));
    }
    let kb_canon = kb_root
        .canonicalize()
        .map_err(|e| format!("kb root: {e}"))?;
    let mut target = local_dir
        .canonicalize()
        .unwrap_or_else(|_| local_dir.clone());
    if !rel_path.is_empty() {
        for comp in Path::new(rel_path).components() {
            match comp {
                std::path::Component::Normal(s) => target.push(s),
                std::path::Component::CurDir => {}
                _ => return Err("invalid path".into()),
            }
        }
    }
    let prefix = format!(
        "{}{}",
        kb_canon.to_string_lossy(),
        std::path::MAIN_SEPARATOR
    );
    let target_str = target.to_string_lossy();
    if !target_str.starts_with(&prefix) && target != kb_canon {
        return Err("path traversal not allowed".into());
    }
    if !target.is_dir() {
        return Err("invalid path".into());
    }
    Ok(target)
}

#[cfg(test)]
#[path = "../unit-tests/repositories/knowledge.rs"]
mod tests;
