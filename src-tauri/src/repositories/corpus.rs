use std::path::{Path, PathBuf};

/// Aligns with `server.py::_kb_safe_path`.
pub fn kb_safe_path(kb_root: &Path, repo: &str, rel_path: &str) -> Result<PathBuf, String> {
    let repo = repo.trim();
    let rel_path = rel_path.trim();
    if repo.is_empty() || !repo.contains('/') {
        return Err("invalid repo format".into());
    }
    if rel_path.is_empty() || rel_path.contains("..") {
        return Err("invalid path".into());
    }
    let repo_name = repo.split('/').next_back().unwrap_or("");
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return Err(format!("repo not cloned locally: {repo_name}"));
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

/// Aligns with `server.py::_kb_annotation_path`.
pub fn kb_annotation_path(kb_root: &Path, repo: &str, rel_path: &str) -> Result<PathBuf, String> {
    kb_safe_path(kb_root, repo, rel_path)?;
    let repo = repo.trim();
    let rel_path = rel_path.trim();
    let repo_name = repo.split('/').next_back().unwrap_or("");
    let mut ann_rel = rel_path.to_string();
    if ann_rel.ends_with(".md") {
        ann_rel = format!("{}.json", &ann_rel[..ann_rel.len() - 3]);
    } else {
        ann_rel.push_str(".json");
    }
    let ann_path = kb_root
        .join(repo_name)
        .join(".knowledge_annotations")
        .join(&ann_rel);
    let kb_root = kb_root.canonicalize().map_err(|e| format!("kb root: {e}"))?;
    let ann_path = ann_path.canonicalize().unwrap_or(ann_path);
    let prefix = format!("{}{}", kb_root.to_string_lossy(), std::path::MAIN_SEPARATOR);
    if !ann_path.to_string_lossy().starts_with(&prefix) {
        return Err("path traversal not allowed".into());
    }
    Ok(ann_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_path_traversal() {
        let dir = tempfile::tempdir().expect("tmp");
        let kb = dir.path().join("kb");
        fs::create_dir_all(kb.join("myrepo")).expect("mkdir");
        let err = kb_safe_path(&kb, "lulufoo/myrepo", "../secret.md").unwrap_err();
        assert_eq!(err, "invalid path");
    }

    #[test]
    fn resolves_file_under_repo() {
        let dir = tempfile::tempdir().expect("tmp");
        let kb = dir.path().join("kb");
        let repo_dir = kb.join("myrepo");
        fs::create_dir_all(&repo_dir).expect("mkdir");
        let md = repo_dir.join("docs/a.md");
        fs::create_dir_all(md.parent().unwrap()).expect("mkdir");
        fs::write(&md, "# hi").expect("write");
        let p = kb_safe_path(&kb, "lulufoo/myrepo", "docs/a.md").expect("ok");
        assert!(p.is_file());
    }
}
