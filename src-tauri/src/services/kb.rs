use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::repositories::corpus::{kb_annotation_path, kb_safe_path};
use crate::services::workbench_read::{categories_from_git_status, knowledge_corpus_root_string};

fn err_status_code(msg: &str) -> u16 {
    if msg.contains("invalid") || msg.contains("traversal") || msg.contains("required") {
        400
    } else {
        404
    }
}

pub fn kb_read_json(repo_root: &Path, repo: &str, path: &str) -> Value {
    let kb_root_str = knowledge_corpus_root_string(repo_root);
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
    let kb_root_str = knowledge_corpus_root_string(repo_root);
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

pub fn kb_status_json(repo_root: &Path, repo: &str) -> Value {
    let repo = repo.trim();
    if repo.is_empty() || !repo.contains('/') {
        return json!({ "error": "repo required", "_status": 400 });
    }
    let repo_name = repo.split('/').next_back().unwrap_or("");
    let kb_root_str = knowledge_corpus_root_string(repo_root);
    let kb_root = Path::new(&kb_root_str);
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return json!({
            "error": format!("repo not cloned: {repo_name}"),
            "_status": 404,
        });
    }
    let stdout = match crate::integrations::git::status_porcelain(&local_dir) {
        Ok(s) => s,
        Err(e) => return json!({ "error": e.message, "_status": 500 }),
    };
    let mut categories = categories_from_git_status(&stdout);
    let ann_dir = local_dir.join(".knowledge_annotations");
    if ann_dir.is_dir() {
        let ignored = crate::integrations::git::exec(&local_dir, &["check-ignore", "-q", ".knowledge_annotations"])
            .map(|o| o.success)
            .unwrap_or(false);
        if ignored {
            if let Ok(ann_out) =
                crate::integrations::git::exec(&local_dir, &["status", "--porcelain", "--ignored", ".knowledge_annotations/"])
            {
                if ann_out.success {
                    for line in ann_out.stdout.lines() {
                        if line.len() < 4 {
                            continue;
                        }
                        let xy = &line[..2];
                        let fpath = line[3..].trim().to_string();
                        let key = if xy == "!!" || xy == "??" {
                            Some("new")
                        } else if xy.starts_with('M') || xy.ends_with('M') {
                            Some("modified")
                        } else if xy.starts_with('D') || xy.ends_with('D') {
                            Some("deleted")
                        } else {
                            None
                        };
                        if let Some(key) = key {
                            if let Some(arr) =
                                categories.get_mut(key).and_then(|v| v.as_array_mut())
                            {
                                arr.push(json!(fpath));
                            }
                        }
                    }
                }
            }
        }
    }
    let total: usize = ["new", "modified", "deleted", "renamed", "conflicted"]
        .iter()
        .filter_map(|k| categories.get(*k).and_then(|v| v.as_array()))
        .map(|a| a.len())
        .sum();
    let ahead = crate::integrations::git::ahead_count(&local_dir).unwrap_or(0);
    categories.insert("total".into(), json!(total));
    categories.insert("ahead".into(), json!(ahead));
    Value::Object(categories)
}

#[cfg(test)]
#[path = "../unit-tests/services/kb.rs"]
mod tests;
