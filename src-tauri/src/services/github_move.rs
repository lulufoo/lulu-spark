//! GitHub Contents API move (`/api/gh-move`).

use base64::Engine;
use serde_json::{json, Value};

use crate::integrations::github;

fn move_single_file(
    src_owner: &str,
    src_repo: &str,
    src_ref: &str,
    src_path: &str,
    dst_owner: &str,
    dst_repo: &str,
    dst_path: &str,
) -> Value {
    let src_info = match github::get_contents(src_owner, src_repo, src_path, Some(src_ref)) {
        Ok(v) => v,
        Err(e) => return json!({ "error": format!("获取源文件失败：{}", e.message) }),
    };
    let content_b64 = src_info
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .replace('\n', "");
    let src_sha = src_info.get("sha").and_then(|v| v.as_str()).unwrap_or("");
    if content_b64.is_empty() || src_sha.is_empty() {
        return json!({ "error": "源文件内容无法读取" });
    }
    let decoded = match base64::engine::general_purpose::STANDARD.decode(content_b64.as_bytes()) {
        Ok(b) => b,
        Err(e) => return json!({ "error": format!("decode: {e}") }),
    };
    let text = String::from_utf8_lossy(&decoded);
    let msg_put = format!("move: {src_repo}/{src_path} → {dst_repo}/{dst_path}");
    if let Err(e) = github::put_contents(dst_owner, dst_repo, dst_path, &msg_put, &text, None) {
        return json!({ "error": format!("创建目标文件失败：{}", e.message) });
    }
    let msg_del = format!("move: remove {src_repo}/{src_path} (moved to {dst_repo}/{dst_path})");
    match github::delete_contents(src_owner, src_repo, src_path, &msg_del, src_sha) {
        Ok(_) => json!({ "ok": true, "dst_path": dst_path }),
        Err(e) => json!({
            "ok": true,
            "warn": format!("目标文件已创建，但删除源文件失败：{}", e.message),
            "dst_path": dst_path
        }),
    }
}

pub fn gh_move_assets(payload: &Value) -> Value {
    let src_url = payload.get("src_url").and_then(|v| v.as_str()).unwrap_or("").trim();
    let dst_dir_url = payload
        .get("dst_dir_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if src_url.is_empty() || dst_dir_url.is_empty() {
        return json!({ "error": "src_url and dst_dir_url required", "_status": 400 });
    }

    let dst_parts = parse_github_dst(dst_dir_url);
    let Some((dst_owner, dst_repo, dst_dir)) = dst_parts else {
        return json!({ "error": "目标目录 URL 格式无效", "_status": 400 });
    };

    if let Some((src_owner, src_repo, src_ref, src_path)) = parse_github_blob(src_url) {
        let filename = src_path.split('/').next_back().unwrap_or(&src_path);
        let dst_path = if dst_dir.is_empty() {
            filename.to_string()
        } else {
            format!("{dst_dir}/{filename}")
        };
        return move_single_file(
            &src_owner, &src_repo, &src_ref, &src_path,
            &dst_owner, &dst_repo, &dst_path,
        );
    }

    if let Some((src_owner, src_repo, src_ref, src_dir)) = parse_github_tree(src_url) {
        let dir_name = src_dir.split('/').next_back().unwrap_or(&src_dir);
        let dst_base = if dst_dir.is_empty() {
            dir_name.to_string()
        } else {
            format!("{dst_dir}/{dir_name}")
        };
        let tree = match github::get_tree_recursive(&src_owner, &src_repo, &src_ref) {
            Ok(t) => t,
            Err(e) => {
                return json!({ "error": format!("获取目录内容失败：{}", e.message), "_status": 500 });
            }
        };
        let prefix = format!("{src_dir}/");
        let files: Vec<String> = tree
            .get("tree")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let ty = item.get("type")?.as_str()?;
                        let path = item.get("path")?.as_str()?;
                        if ty == "blob"
                            && path.starts_with(&prefix)
                            && path.split('/').next_back() != Some(".gitkeep")
                        {
                            Some(path.to_string())
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        if files.is_empty() {
            return json!({ "error": format!("目录为空或不存在：{src_dir}"), "_status": 400 });
        }
        let mut moved = Vec::new();
        let mut failed = Vec::new();
        for file_path in files {
            let rel = file_path.strip_prefix(&prefix).unwrap_or(&file_path);
            let dst_path = format!("{dst_base}/{rel}");
            let result = move_single_file(
                &src_owner, &src_repo, &src_ref, &file_path,
                &dst_owner, &dst_repo, &dst_path,
            );
            if result.get("ok") == Some(&json!(true)) {
                moved.push(dst_path);
            } else {
                failed.push(json!({
                    "src": file_path,
                    "error": result.get("error").and_then(|v| v.as_str()).unwrap_or("unknown")
                }));
            }
        }
        if moved.is_empty() && !failed.is_empty() {
            let first = failed[0]["error"].as_str().unwrap_or("unknown");
            return json!({ "error": format!("所有文件移动失败，第一个错误：{first}"), "_status": 500 });
        }
        return json!({
            "ok": true,
            "dst_path": dst_base,
            "moved": moved.len(),
            "failed": failed,
            "warn": if failed.is_empty() { Value::Null } else { json!(format!("{} 个文件移动失败", failed.len())) }
        });
    }

    json!({
        "error": "源 URL 格式无效，需为 GitHub blob（文件）或 tree（目录）链接",
        "_status": 400
    })
}

fn parse_github_blob(url: &str) -> Option<(String, String, String, String)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 5 || parts[2] != "blob" {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[3].to_string(),
        parts[4..].join("/"),
    ))
}

fn parse_github_tree(url: &str) -> Option<(String, String, String, String)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 5 || parts[2] != "tree" {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[3].to_string(),
        parts[4..].join("/").trim_end_matches('/').to_string(),
    ))
}

/// Last path segment looks like a file (e.g. `rules_sync.json`, `design.md`).
fn path_ends_with_file_segment(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let Some(ext) = name.rsplit_once('.').map(|(_, ext)| ext) else {
        return false;
    };
    !ext.is_empty()
        && ext.len() <= 16
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
        && ext.chars().any(|c| c.is_alphabetic())
}

/// Strip `tree/{ref}/…` or `blob/{ref}/…` to a repo-relative path.
/// Returns `(path, is_blob_link)`.
fn strip_github_tree_or_blob_prefix(tail: &str) -> Option<(String, bool)> {
    let (rest, is_blob) = if let Some(r) = tail.strip_prefix("tree/") {
        (r, false)
    } else if let Some(r) = tail.strip_prefix("blob/") {
        (r, true)
    } else {
        return None;
    };
    let path = if let Some(idx) = rest.find('/') {
        rest[idx + 1..].to_string()
    } else {
        String::new()
    };
    Some((path, is_blob))
}

fn parse_github_dst(url: &str) -> Option<(String, String, String)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let mut parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    parts.drain(0..2);
    let tail = parts.join("/");

    let dir = match strip_github_tree_or_blob_prefix(&tail) {
        Some((path, is_blob)) => {
            if is_blob && !path.is_empty() && path_ends_with_file_segment(&path) {
                // blob 文件链接：目标为其所在目录
                path.rfind('/')
                    .map(|i| path[..i].to_string())
                    .unwrap_or_default()
            } else {
                path
            }
        }
        None => tail,
    };
    Some((owner, repo, dir.trim_matches('/').to_string()))
}

#[cfg(test)]
#[path = "../unit-tests/services/github_move.rs"]
mod tests;
