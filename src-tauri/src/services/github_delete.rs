//! GitHub Contents API delete (`/api/gh-delete`).

use serde_json::{json, Value};

use crate::integrations::github;
use crate::services::github_url::*;

pub(crate) fn collect_tree_blob_paths(tree: &Value, dir: &str) -> Vec<String> {
    let prefix = format!("{dir}/");
    let mut files: Vec<String> = tree
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
    files.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| b.cmp(a)));
    files
}

fn delete_single_file(owner: &str, repo: &str, ref_name: &str, path: &str) -> Result<(), String> {
    let info = match github::get_contents(owner, repo, path, Some(ref_name)) {
        Ok(v) => v,
        Err(e) => return Err(format!("获取源文件失败：{}", e.message)),
    };
    let sha = info.get("sha").and_then(|v| v.as_str()).unwrap_or("");
    if sha.is_empty() {
        return Err("源文件 sha 无法读取".into());
    }
    let msg = format!("delete: {repo}/{path}");
    github::delete_contents(owner, repo, path, &msg, sha)
        .map(|_| ())
        .map_err(|e| e.message)
}

pub fn gh_delete_assets(payload: &Value) -> Value {
    let url = payload.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
    if url.is_empty() {
        return json!({ "error": "url required", "_status": 400 });
    }

    if let Some((owner, repo, ref_name, path)) = parse_github_blob(url) {
        match delete_single_file(&owner, &repo, &ref_name, &path) {
            Ok(()) => json!({ "ok": true, "deleted": 1 }),
            Err(e) => json!({ "error": e }),
        }
    } else if let Some((owner, repo, ref_name, dir)) = parse_github_tree(url) {
        let tree = match github::get_tree_recursive(&owner, &repo, &ref_name) {
            Ok(t) => t,
            Err(e) => {
                return json!({ "error": format!("获取目录内容失败：{}", e.message), "_status": 500 });
            }
        };
        let files = collect_tree_blob_paths(&tree, &dir);
        if files.is_empty() {
            return json!({ "error": format!("目录为空或不存在：{dir}"), "_status": 400 });
        }
        let mut deleted = 0usize;
        let mut failed = Vec::new();
        for file_path in files {
            match delete_single_file(&owner, &repo, &ref_name, &file_path) {
                Ok(()) => deleted += 1,
                Err(e) => failed.push(json!({ "src": file_path, "error": e })),
            }
        }
        if deleted == 0 && !failed.is_empty() {
            let first = failed[0]["error"].as_str().unwrap_or("unknown");
            return json!({ "error": format!("所有文件删除失败，第一个错误：{first}"), "_status": 500 });
        }
        json!({
            "ok": true,
            "deleted": deleted,
            "failed": failed,
            "warn": if failed.is_empty() { Value::Null } else { json!(format!("{} 个文件删除失败", failed.len())) }
        })
    } else {
        json!({
            "error": "源 URL 格式无效，需为 GitHub blob（文件）或 tree（目录）链接",
            "_status": 400
        })
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/github_delete.rs"]
mod tests;
