//! Create and delete a knowledge file or folder.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::{kb_dir_name, knowledge_dir_path, knowledge_file_path};
use crate::services::spark_data::{
    self, directory_id_for_folder, register_knowledge_doc, remove_knowledge_paths,
};

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

/// Records follow the files: written only after the file operation succeeded.
fn register_doc(repo: &str, rel: &str) -> Result<(), String> {
    let conn = spark_data::open()?;
    let directory_id = directory_id_for_folder(&conn, kb_dir_name(repo)?)?;
    register_knowledge_doc(&conn, &directory_id, rel).map(|_| ())
}

fn remove_docs(repo: &str, rel: &str) -> Result<(), String> {
    let conn = spark_data::open()?;
    let directory_id = directory_id_for_folder(&conn, kb_dir_name(repo)?)?;
    remove_knowledge_paths(&conn, &directory_id, rel).map(|_| ())
}

fn resolve_parent_dir(kb_root: &Path, repo: &str, parent: &str) -> Result<PathBuf, String> {
    knowledge_dir_path(kb_root, kb_dir_name(repo)?, parent)
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
    let dest = match kb_dir_name(&repo).and_then(|folder| knowledge_file_path(&kb_root, folder, &dest_rel)) {
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
    if !is_dir {
        if let Err(e) = register_doc(&repo, &dest_rel) {
            return json!({ "error": e, "_status": 500 });
        }
    }
    json!({ "ok": true, "path": dest_rel })
}

pub fn kb_delete(repo_root: &Path, repo: String, path: String) -> Value {
    let kb_root = kb_root_path(repo_root);
    let src_rel = path.trim();
    if src_rel.is_empty() {
        return json!({ "error": "invalid path", "_status": 400 });
    }
    let src = match kb_dir_name(&repo).and_then(|folder| knowledge_file_path(&kb_root, folder, src_rel)) {
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
    if let Err(e) = remove_docs(&repo, src_rel) {
        return json!({ "error": e, "_status": 500 });
    }
    json!({ "ok": true, "path": src_rel })
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/entry.rs"]
mod tests;
