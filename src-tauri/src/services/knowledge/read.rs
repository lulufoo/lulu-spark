use std::fs;
use std::path::Path;

use regex::Regex;
use serde_json::{json, Value};

use crate::repositories::knowledge::{kb_annotation_path, kb_list_dir, kb_safe_path};
use crate::services::sediment_kb;
use crate::services::spark_read::knowledge_root_string;

fn err_status_code(msg: &str) -> u16 {
    if msg.contains("invalid") || msg.contains("traversal") || msg.contains("required") {
        400
    } else {
        404
    }
}

pub fn kb_read_json(repo_root: &Path, repo: &str, path: &str) -> Value {
    let kb_root_str = knowledge_root_string(repo_root);
    let kb_root = Path::new(&kb_root_str);
    match kb_safe_path(kb_root, repo, path) {
        Err(e) => json!({ "error": e, "_status": err_status_code(&e) }),
        Ok(target) => {
            if !target.is_file() {
                let e = format!("file not found: {}", path.trim());
                return json!({ "error": e, "_status": 404 });
            }
            let content = fs::read_to_string(&target).unwrap_or_default();
            json!({ "content": content })
        }
    }
}

pub fn kb_annotation_json(repo_root: &Path, repo: &str, path: &str) -> Value {
    let kb_root_str = knowledge_root_string(repo_root);
    let kb_root = Path::new(&kb_root_str);
    match kb_annotation_path(kb_root, repo, path) {
        Err(e) => json!({ "error": e, "_status": err_status_code(&e) }),
        Ok(ann_path) => {
            if !ann_path.is_file() {
                return json!({});
            }
            let Ok(text) = fs::read_to_string(&ann_path) else {
                return json!({});
            };
            serde_json::from_str(&text).unwrap_or_else(|_| json!({}))
        }
    }
}

pub fn kb_list_json(repo_root: &Path, repo: &str, path: &str, mode: &str) -> Value {
    if mode != "flat" {
        return json!({
            "error": "mode tree not implemented",
            "_status": 400,
        });
    }
    let kb_root_str = knowledge_root_string(repo_root);
    let kb_root = Path::new(&kb_root_str);
    match kb_list_dir(kb_root, repo, path) {
        Err(e) => json!({ "error": e, "_status": err_status_code(&e) }),
        Ok(dir) => {
            let mut entries: Vec<Value> = Vec::new();
            let read_dir = match fs::read_dir(&dir) {
                Ok(rd) => rd,
                Err(e) => {
                    return json!({
                        "error": format!("read dir: {e}"),
                        "_status": 500,
                    });
                }
            };
            let base_path = path.trim();
            for entry in read_dir.flatten() {
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy().to_string();
                let relative_path = if base_path.is_empty() {
                    name.clone()
                } else {
                    format!("{base_path}/{name}")
                };
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                entries.push(json!({
                    "name": name,
                    "relative_path": relative_path,
                    "is_dir": is_dir,
                }));
            }
            entries.sort_by(|a, b| {
                let a_dir = a["is_dir"].as_bool().unwrap_or(false);
                let b_dir = b["is_dir"].as_bool().unwrap_or(false);
                match (a_dir, b_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a["name"]
                        .as_str()
                        .unwrap_or("")
                        .cmp(b["name"].as_str().unwrap_or("")),
                }
            });
            Value::Array(entries)
        }
    }
}

fn count_md_in_repo_dir(kb_root: &Path, repo_name: &str, hide: &[Regex]) -> usize {
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return 0;
    }
    count_md_recursive(&local_dir, hide)
}

fn count_md_recursive(dir: &Path, hide: &[Regex]) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if super::name_is_hidden(&name, hide) {
            continue;
        }
        if path.is_dir() {
            count += count_md_recursive(&path, hide);
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            count += 1;
        }
    }
    count
}

pub fn kb_doc_count_json(
    repo_root: &Path,
    repo: &str,
    category_id: Option<&str>,
) -> Value {
    let hide = super::compiled_hide_regexes();
    let kb_root_str = knowledge_root_string(repo_root);
    let kb_root = Path::new(&kb_root_str);

    if let Some(cat_id) = category_id.map(str::trim).filter(|s| !s.is_empty()) {
        let repos = match sediment_kb::load_repos() {
            Ok(repos) => repos,
            Err(e) => return json!({ "error": e.to_string(), "_status": 500 }),
        };
        let count: usize = repos
            .repos
            .iter()
            .filter(|r| r.category_id == cat_id)
            .map(|r| {
                let repo_name = r.full_name.split('/').next_back().unwrap_or("");
                count_md_in_repo_dir(kb_root, repo_name, &hide)
            })
            .sum();
        return json!({ "count": count });
    }

    let repo = repo.trim();
    if repo.is_empty() || repo.contains("..") || repo.contains('\\') || repo.contains('\0') {
        return json!({ "error": "invalid directory name", "_status": 400 });
    }
    let repo_name = repo.split('/').next_back().unwrap_or("");
    if repo_name.is_empty() || repo_name == "." || repo_name == ".." {
        return json!({ "error": "invalid directory name", "_status": 400 });
    }
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return json!({
            "error": format!("directory not found: {repo_name}"),
            "_status": 404,
        });
    }
    json!({
        "count": count_md_in_repo_dir(kb_root, repo_name, &hide),
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/knowledge/read.rs"]
mod tests;
