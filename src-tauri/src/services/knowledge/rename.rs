//! Same-directory rename of a knowledge file or folder.

use std::fs;
use std::path::{Component, Path, PathBuf};

use serde_json::{json, Value};

use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::kb_safe_path;
use crate::services::id::random_hex12;

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

fn next_relative_path(from: &str, name: &str) -> Option<String> {
    let from = from.trim();
    if from.is_empty() {
        return None;
    }
    match from.rfind('/') {
        Some(i) => Some(format!("{}/{}", &from[..i], name)),
        None => Some(name.to_string()),
    }
}

fn same_inode(src: &Path, dest: &Path) -> bool {
    match (src.canonicalize(), dest.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn rename_path(src: &Path, dest: &Path) -> Result<(), String> {
    if dest.exists() && same_inode(src, dest) {
        if src.file_name() == dest.file_name() {
            return Ok(());
        }
        let tmp = dest.with_file_name(format!(".{}.rename", random_hex12()));
        fs::rename(src, &tmp).map_err(|e| e.to_string())?;
        return fs::rename(&tmp, dest).map_err(|e| e.to_string());
    }
    fs::rename(src, dest).map_err(|e| e.to_string())
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

fn move_if_absent(src: &Path, dest: &Path) {
    if !src.exists() || dest.exists() {
        return;
    }
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::rename(src, dest);
}

fn move_annotation_sidecars(kb_root: &Path, repo: &str, from: &str, to: &str, is_dir: bool) {
    if is_dir {
        if let (Ok(old_dir), Ok(new_dir)) = (
            annotation_tree_dir(kb_root, repo, from),
            annotation_tree_dir(kb_root, repo, to),
        ) {
            move_if_absent(&old_dir, &new_dir);
        }
    }
    if let (Ok(old_ann), Ok(new_ann)) = (
        annotation_sidecar_path(kb_root, repo, from),
        annotation_sidecar_path(kb_root, repo, to),
    ) {
        move_if_absent(&old_ann, &new_ann);
    }
}

pub fn kb_rename(repo_root: &Path, repo: String, path: String, name: String) -> Value {
    let kb_root = kb_root_path(repo_root);
    let src_rel = path.trim();
    let new_name = name.trim();
    if let Err(e) = validate_entry_name(new_name) {
        return json!({ "error": e, "_status": 400 });
    }
    let Some(dest_rel) = next_relative_path(src_rel, new_name) else {
        return json!({ "error": "invalid path", "_status": 400 });
    };
    let src = match kb_safe_path(&kb_root, &repo, src_rel) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    if !src.exists() {
        return json!({ "error": format!("not found: {src_rel}"), "_status": 404 });
    }
    if dest_rel == src_rel {
        return json!({ "ok": true, "path": dest_rel });
    }
    let dest = match kb_safe_path(&kb_root, &repo, &dest_rel) {
        Ok(p) => p,
        Err(e) => return json!({ "error": e, "_status": err_status(&e) }),
    };
    if dest.exists() && !same_inode(&src, &dest) {
        return json!({ "error": "destination exists", "_status": 409 });
    }
    let is_dir = src.is_dir();
    if let Err(e) = rename_path(&src, &dest) {
        return json!({ "error": e, "_status": 500 });
    }
    move_annotation_sidecars(&kb_root, &repo, src_rel, &dest_rel, is_dir);
    json!({ "ok": true, "path": dest_rel })
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/rename.rs"]
mod tests;
