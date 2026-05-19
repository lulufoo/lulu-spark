//! GitHub read-only helpers via `gh` CLI (aligned with `server.py` GET handlers).

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use base64::Engine;
use chrono::{FixedOffset, Utc};
use serde_json::{json, Value};

use crate::services::workbench_read::get_topics;

fn repo_list_cache_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(".cache").join("repo-list.json")
}

pub fn topic_repos(repo_root: &Path) -> HashSet<String> {
    let topics = get_topics(repo_root);
    let mut set = HashSet::new();
    if let Some(arr) = topics.get("topics").and_then(|v| v.as_array()) {
        for t in arr {
            if let Some(r) = t.get("repo").and_then(|v| v.as_str()) {
                set.insert(r.to_string());
            }
        }
    }
    set
}

fn require_known_repo(repo_root: &Path, repo: &str, invalid_msg: &str) -> Result<(), Value> {
    let repo = repo.trim();
    if repo.is_empty() || !repo.contains('/') {
        return Err(json!({ "error": invalid_msg, "_status": 400 }));
    }
    if !topic_repos(repo_root).contains(repo) {
        return Err(json!({
            "error": format!("Unknown repo: {repo}"),
            "_status": 400,
        }));
    }
    Ok(())
}

/// `gh --paginate` may concatenate multiple JSON arrays in one stdout buffer.
pub fn parse_concatenated_json_arrays(raw: &str) -> Vec<Value> {
    let raw = raw.trim();
    let mut all = Vec::new();
    for result in serde_json::Deserializer::from_str(raw).into_iter::<Value>() {
        match result {
            Ok(Value::Array(items)) => all.extend(items),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    if all.is_empty() && !raw.is_empty() {
        if let Ok(Value::Array(items)) = serde_json::from_str(raw) {
            all = items;
        }
    }
    all
}

pub fn parse_repo_dirs(stdout: &str) -> Result<Value, String> {
    let items: Vec<Value> = serde_json::from_str(stdout).map_err(|e| e.to_string())?;
    let mut dirs: Vec<String> = items
        .iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?;
            let ty = item.get("type")?.as_str()?;
            if ty == "dir" && !name.starts_with('.') && !name.starts_with('_') {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect();
    dirs.sort();
    Ok(json!({ "dirs": dirs }))
}

fn fetch_type_meta(full_name: &str, default_branch: &str) -> (String, Option<String>, Option<String>) {
    let branch = if default_branch.is_empty() {
        "main"
    } else {
        default_branch
    };
    let path = format!(
        "repos/{full_name}/contents/.repository-type.json?ref={}",
        urlencoding::encode(branch)
    );
    let r = Command::new("gh")
        .args([
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            &path,
        ])
        .output();
    let Ok(out) = r else {
        return (full_name.to_string(), None, None);
    };
    if !out.status.success() {
        return (full_name.to_string(), None, None);
    }
    let Ok(meta) = serde_json::from_slice::<Value>(&out.stdout) else {
        return (full_name.to_string(), None, None);
    };
    let content = meta.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let cleaned = content.replace('\n', "");
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(cleaned.as_bytes()) else {
        return (full_name.to_string(), None, None);
    };
    let Ok(doc) = serde_json::from_slice::<Value>(&bytes) else {
        return (full_name.to_string(), None, None);
    };
    (
        full_name.to_string(),
        doc.get("type").and_then(|v| v.as_str()).map(str::to_string),
        doc
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    )
}

fn cached_at_label() -> String {
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    Utc::now()
        .with_timezone(&offset)
        .format("%m月%d日 %H:%M")
        .to_string()
}

pub fn check_file_json(repo_root: &Path, repo: &str, path: &str) -> Value {
    if let Err(v) = require_known_repo(repo_root, repo, "repo and path required") {
        return v;
    }
    let path = path.trim();
    if path.is_empty() {
        return json!({ "error": "repo and path required", "_status": 400 });
    }
    let repo = repo.trim();
    let (owner, repo_name) = match repo.split_once('/') {
        Some(p) => p,
        None => return json!({ "error": "repo and path required", "_status": 400 }),
    };
    let api_path = format!("repos/{owner}/{repo_name}/contents/{path}");
    let out = Command::new("gh").args(["api", &api_path]).output();
    let Ok(out) = out else {
        return json!({ "error": "gh failed", "_status": 500 });
    };
    json!({ "exists": out.status.success() })
}

pub fn repo_dirs_json(repo_root: &Path, repo: &str) -> Value {
    if let Err(v) = require_known_repo(repo_root, repo, "missing or invalid repo") {
        return v;
    }
    let repo = repo.trim();
    let (owner, repo_name) = match repo.split_once('/') {
        Some(p) => p,
        None => return json!({ "error": "missing or invalid repo", "_status": 400 }),
    };
    let api_path = format!("repos/{owner}/{repo_name}/contents/");
    let out = Command::new("gh")
        .args(["api", &api_path])
        .output();
    let Ok(out) = out else {
        return json!({ "error": "gh failed", "_status": 500 });
    };
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let msg = if err.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err.trim().to_string()
        };
        return json!({ "error": msg, "_status": 500 });
    }
    match parse_repo_dirs(&String::from_utf8_lossy(&out.stdout)) {
        Ok(v) => v,
        Err(e) => json!({ "error": e, "_status": 500 }),
    }
}

pub fn repo_list_json(repo_root: &Path, force: bool) -> Value {
    let cache_path = repo_list_cache_path(repo_root);
    if !force && cache_path.is_file() {
        if let Ok(text) = fs::read_to_string(&cache_path) {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                return v;
            }
        }
    }

    let out = Command::new("gh")
        .args([
            "api",
            "-H",
            "Accept: application/vnd.github+json",
            "user/repos?per_page=100&affiliation=owner",
            "--paginate",
        ])
        .output();
    let Ok(out) = out else {
        return json!({ "error": "gh failed", "_status": 500 });
    };
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let msg = if err.trim().is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err.trim().to_string()
        };
        return json!({ "error": msg, "_status": 500 });
    }

    let all_repos = parse_concatenated_json_arrays(&String::from_utf8_lossy(&out.stdout));
    let mut type_map: std::collections::HashMap<String, (Option<String>, Option<String>)> =
        std::collections::HashMap::new();
    for repo in &all_repos {
        let full_name = repo.get("full_name").and_then(|v| v.as_str()).unwrap_or("");
        let branch = repo
            .get("default_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main");
        let (name, ty, desc) = fetch_type_meta(full_name, branch);
        type_map.insert(name, (ty, desc));
    }

    let mut repos_out = Vec::new();
    for repo in all_repos {
        let full_name = repo.get("full_name").and_then(|v| v.as_str()).unwrap_or("");
        let (repo_type, description) = type_map
            .get(full_name)
            .cloned()
            .unwrap_or((None, None));
        repos_out.push(json!({
            "name": repo.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            "full_name": full_name,
            "type": repo_type,
            "description": description,
        }));
    }

    let payload = json!({
        "repos": repos_out,
        "cached_at": cached_at_label(),
    });

    if let Some(parent) = cache_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &cache_path,
        serde_json::to_string_pretty(&payload).unwrap_or_default(),
    );

    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_concatenated_arrays_merges() {
        let raw = r#"[{"full_name":"a/b"}][{"full_name":"c/d"}]"#;
        let items = parse_concatenated_json_arrays(raw);
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parse_repo_dirs_filters_dot_dirs() {
        let stdout = r#"[
          {"name": "docs", "type": "dir"},
          {"name": ".hidden", "type": "dir"},
          {"name": "readme.md", "type": "file"}
        ]"#;
        let v = parse_repo_dirs(stdout).expect("parse");
        assert_eq!(v["dirs"], json!(["docs"]));
    }

    #[test]
    fn check_file_rejects_unknown_repo() {
        let dir = tempfile::tempdir().expect("tmp");
        let v = check_file_json(dir.path(), "unknown/foo", "a.md");
        assert!(v.get("error").is_some());
    }
}
