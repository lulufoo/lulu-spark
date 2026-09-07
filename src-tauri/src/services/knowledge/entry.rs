//! Create and delete a knowledge file or folder.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::{kb_list_dir, kb_safe_path};

fn err_status(msg: &str) -> u16 {
    if msg.contains("invalid") || msg.contains("traversal") {
        400
    } else if msg.contains("not cloned") || msg.contains("not found") {
        404
    } else {
        500
    }
}

fn kb_root_path(repo_root: &Path) -> PathBuf {
    PathBuf::from(knowledge_root_string(repo_root))
}

fn validate_entry_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." {
        return Err("invalid name".into());
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') || name.contains("..") {
        return Err("invalid name".into());
    }
    Ok(())
}

fn normalize_file_name(name: &str) -> Result<String, String> {
    validate_entry_name(name)?;
    if name.to_ascii_lowercase().ends_with(".md") {
        return Ok(format!("{}.md", &name[..name.len() - 3]));
    }
    if name.contains('.') {
        return Err("file must be .md".into());
    }
    Ok(format!("{name}.md"))
}

fn join_parent(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

fn annotation_rel(rel: &str) -> String {
    if rel.ends_with(".md") {
        format!("{}.json", &rel[..rel.len() - 3])
    } else {
        format!("{rel}.json")
    }
}

fn annotation_base(kb_root: &Path, repo: &str, rel: &str) -> Result<PathBuf, String> {
    kb_safe_path(kb_root, repo, rel)?;
    let repo_name = repo.trim().split('/').next_back().unwrap_or("");
    let kb_canon = kb_root.canonicalize().map_err(|e| format!("kb root: {e}"))?;
    Ok(kb_canon.join(repo_name).join(".knowledge_annotations"))
}

fn push_rel(mut path: PathBuf, rel: &str) -> Result<PathBuf, String> {
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(s) => path.push(s),
            Component::CurDir => {}
            _ => return Err("invalid path".into()),
        }
    }
    Ok(path)
}

fn annotation_tree_dir(kb_root: &Path, repo: &str, rel: &str) -> Result<PathBuf, String> {
    push_rel(annotation_base(kb_root, repo, rel)?, rel.trim())
}

fn annotation_sidecar_path(kb_root: &Path, repo: &str, rel: &str) -> Result<PathBuf, String> {
    push_rel(annotation_base(kb_root, repo, rel)?, &annotation_rel(rel.trim()))
}

fn remove_if_exists(path: &Path) {
    if path.is_dir() {
        let _ = fs::remove_dir_all(path);
    } else if path.is_file() {
        let _ = fs::remove_file(path);
    }
}

fn remove_annotation_sidecars(kb_root: &Path, repo: &str, rel: &str, is_dir: bool) {
    if is_dir {
        if let Ok(dir) = annotation_tree_dir(kb_root, repo, rel) {
            remove_if_exists(&dir);
        }
    }
    if let Ok(ann) = annotation_sidecar_path(kb_root, repo, rel) {
        remove_if_exists(&ann);
    }
}

fn resolve_parent_dir(kb_root: &Path, repo: &str, parent: &str) -> Result<PathBuf, String> {
    kb_list_dir(kb_root, repo, parent)
}

pub fn kb_create(
    repo_root: &Path,
    repo: String,
    parent: String,
    name: String,
    kind: String,
) -> Value {
    let kb_root = kb_root_path(repo_root);
    let parent = parent.trim();
    let kind = kind.trim();
    let is_dir = match kind {
        "dir" => true,
        "file" => false,
        _ => return json!({ "error": "invalid kind", "_status": 400 }),
    };
    let entry_name = if is_dir {
        if let Err(e) = validate_entry_name(name.trim()) {
            return json!({ "error": e, "_status": 400 });
        }
        name.trim().to_string()
    } else {
        match normalize_file_name(name.trim()) {
            Ok(n) => n,
            Err(e) => return json!({ "error": e, "_status": 400 }),
        }
    };
    if let Err(e) = resolve_parent_dir(&kb_root, &repo, parent) {
        return json!({ "error": e, "_status": err_status(&e) });
    }
    let dest_rel = join_parent(parent, &entry_name);
    let dest = match kb_safe_path(&kb_root, &repo, &dest_rel) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    if dest.exists() {
        return json!({ "error": "destination exists", "_status": 409 });
    }
    let result = if is_dir {
        fs::create_dir(&dest)
    } else {
        fs::write(&dest, "")
    };
    if let Err(e) = result {
        return json!({ "error": e.to_string(), "_status": 500 });
    }
    json!({ "ok": true, "path": dest_rel })
}

pub fn kb_delete(repo_root: &Path, repo: String, path: String) -> Value {
    let kb_root = kb_root_path(repo_root);
    let src_rel = path.trim();
    if src_rel.is_empty() {
        return json!({ "error": "invalid path", "_status": 400 });
    }
    let src = match kb_safe_path(&kb_root, &repo, src_rel) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    if !src.exists() {
        return json!({ "error": format!("not found: {src_rel}"), "_status": 404 });
    }
    let is_dir = src.is_dir();
    let result = if is_dir {
        fs::remove_dir_all(&src)
    } else {
        fs::remove_file(&src)
    };
    if let Err(e) = result {
        return json!({ "error": e.to_string(), "_status": 500 });
    }
    remove_annotation_sidecars(&kb_root, &repo, src_rel, is_dir);
    json!({ "ok": true, "path": src_rel })
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/entry.rs"]
mod tests;
