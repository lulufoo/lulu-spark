//! Generic path allow/deny lists for Host file tools. No business names.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathFence {
    pub read_allow: Vec<PathBuf>,
    pub read_deny: Vec<PathBuf>,
    pub write_allow: Vec<PathBuf>,
    pub write_deny: Vec<PathBuf>,
    /// Parent of per-session scratch. Resolved at turn time; not a write root by itself.
    pub scratch_parent: Option<PathBuf>,
}

impl PathFence {
    /// Session scratch directory: `{scratch_parent}/{session_id}`. `None` when
    /// this fence has no scratch parent.
    pub fn session_scratch_root(&self, session_id: &str) -> Result<Option<PathBuf>, String> {
        let Some(parent) = &self.scratch_parent else {
            return Ok(None);
        };
        let segment = sanitize_session_segment(session_id)?;
        Ok(Some(stored_path(parent.join(segment))))
    }

    pub fn with_session_scratch(&self, session_id: &str) -> Result<Self, String> {
        let Some(scratch) = self.session_scratch_root(session_id)? else {
            return Ok(self.clone());
        };
        let mut next = self.clone();
        if !next.read_allow.iter().any(|p| p == &scratch) {
            next.read_allow.push(scratch.clone());
        }
        next.write_allow = vec![scratch];
        Ok(next)
    }

    pub fn allows_read(&self, path: &Path) -> bool {
        in_any(path, &self.read_allow) && !in_any(path, &self.read_deny)
    }

    pub fn allows_write(&self, path: &Path) -> bool {
        in_any(path, &self.write_allow)
            && !in_any(path, &self.write_deny)
            && !in_any(path, &self.read_deny)
    }

    pub fn is_read_denied(&self, path: &Path) -> bool {
        in_any(path, &self.read_deny)
    }

    /// Directory write roots only (session scratch). Exact-file grants are not workspace.
    pub fn is_session_workspace(&self, path: &Path) -> bool {
        self.write_allow
            .iter()
            .any(|root| !root.is_file() && is_under(path, root))
    }

    /// Exact staged path: readable and writable. Does not grant siblings.
    pub fn grant_staged_file(&mut self, path: PathBuf) {
        if !self.read_allow.iter().any(|root| root == &path) {
            self.read_allow.push(path.clone());
        }
        if !self.write_allow.iter().any(|root| root == &path) {
            self.write_allow.push(path);
        }
    }
}

pub fn sanitize_session_segment(session_id: &str) -> Result<&str, String> {
    let id = session_id.trim();
    if id.is_empty() || id.len() > 80 {
        return Err("invalid session scratch id".into());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("invalid session scratch id".into());
    }
    Ok(id)
}

pub fn stored_path(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn existing_regular_file(path: &str) -> Result<PathBuf, String> {
    let abs = require_absolute(path)?;
    let meta = std::fs::symlink_metadata(&abs).map_err(|_| "path does not exist".to_string())?;
    if meta.file_type().is_dir() {
        return Err("path must be a regular file".into());
    }
    let canon = std::fs::canonicalize(&abs).map_err(|_| "path does not exist".to_string())?;
    if !canon.is_file() {
        return Err("path must be a regular file".into());
    }
    Ok(canon)
}

/// Absolute existing regular file eligible to register on Stage (never a directory).
/// Rejects the session write root and read_deny. Does not require current read allow.
pub fn validate_stage_file(path: &str, fence: &PathFence) -> Result<PathBuf, String> {
    let canon = existing_regular_file(path)?;
    if fence.is_session_workspace(&canon) {
        return Err("path must be outside SESSION_WORKSPACE_DIR".into());
    }
    if fence.is_read_denied(&canon) {
        return Err("path is outside the read fence".into());
    }
    Ok(stored_path(canon))
}

/// Absolute existing regular file under the current read fence (never a directory).
pub fn validate_readable_regular_file(path: &str, fence: &PathFence) -> Result<PathBuf, String> {
    let canon = existing_regular_file(path)?;
    if !fence.allows_read(&canon) {
        return Err("path is outside the read fence".into());
    }
    Ok(stored_path(canon))
}

pub fn require_absolute(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("path is required".into());
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("path must be absolute".into());
    }
    Ok(lexical_abs(&path))
}

fn lexical_abs(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out = PathBuf::from(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn in_any(path: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| is_under(path, root))
}

fn is_under(path: &Path, root: &Path) -> bool {
    let path_cmp = resolve_existing_prefix(path);
    let root_cmp = resolve_existing_prefix(root);
    path_cmp == root_cmp || path_cmp.starts_with(&root_cmp)
}

/// Canonicalize the longest existing prefix, then append the missing suffix.
/// Lets a not-yet-created scratch file compare equal to a canonicalized write root
/// on platforms where `/var` and `/private/var` are the same directory.
fn resolve_existing_prefix(path: &Path) -> PathBuf {
    let lex = lexical_abs(path);
    if let Ok(canonical) = lex.canonicalize() {
        return canonical;
    }
    let mut suffix = Vec::new();
    let mut current = lex.as_path();
    loop {
        if let Ok(canonical) = current.canonicalize() {
            let mut out = canonical;
            for part in suffix.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match current.file_name() {
            Some(name) => {
                suffix.push(name.to_os_string());
                match current.parent() {
                    Some(parent) if !parent.as_os_str().is_empty() => current = parent,
                    _ => break,
                }
            }
            None => break,
        }
    }
    lex
}

#[cfg(test)]
#[path = "../../unit-tests/services/path_fence_tests.rs"]
mod tests;
