//! Same-directory rename of a knowledge file or folder.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::config::roots::knowledge_root_string;
use crate::repositories::knowledge::{kb_dir_name, kb_safe_path};
use crate::services::id::random_hex12;
use crate::services::spark_data::{self, move_knowledge_paths};

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
    if let Err(e) = rename_path(&src, &dest) {
        return json!({ "error": e, "_status": 500 });
    }
    // File first, record second (decision 11). A crash in between is not repaired:
    // the old record turns `missing` on the next scan and the file registers anew.
    let moved = kb_dir_name(&repo).and_then(|repo_dir| {
        let conn = spark_data::open()?;
        move_knowledge_paths(&conn, repo_dir, src_rel, &dest_rel)
    });
    if let Err(e) = moved {
        return json!({ "error": e, "_status": 500 });
    }
    json!({ "ok": true, "path": dest_rel })
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/rename.rs"]
mod tests;
