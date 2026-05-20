//! GitHub read-only helpers via REST (`integrations/github.rs`).

use std::collections::HashMap;
use std::fs;

use chrono::{FixedOffset, Utc};
use serde_json::{json, Value};

use crate::config::paths;
use crate::integrations::github::{self, GithubError};
use crate::services::workbench_read::get_topics;

fn repo_list_cache_path(repo_root: &std::path::Path) -> std::path::PathBuf {
    let _ = repo_root;
    paths::repo_list_cache_path()
        .unwrap_or_else(|_| repo_root.join(".cache").join("repo-list.json"))
}

pub fn topic_repos(repo_root: &std::path::Path) -> std::collections::HashSet<String> {
    let topics = get_topics(repo_root);
    let mut set = std::collections::HashSet::new();
    if let Some(arr) = topics.get("topics").and_then(|v| v.as_array()) {
        for t in arr {
            if let Some(r) = t.get("repo").and_then(|v| v.as_str()) {
                set.insert(r.to_string());
            }
        }
    }
    set
}

fn require_known_repo(repo_root: &std::path::Path, repo: &str, invalid_msg: &str) -> Result<(), Value> {
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

fn gh_err_value(err: GithubError) -> Value {
    github::error_json(&err)
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

pub fn parse_repo_dirs(items: &Value) -> Result<Value, String> {
    let items = match items {
        Value::Array(a) => a,
        _ => return Err("expected array".into()),
    };
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
    let (owner, repo) = match full_name.split_once('/') {
        Some(p) => p,
        None => return (full_name.to_string(), None, None),
    };
    let Ok(meta) = github::get_contents(owner, repo, ".repository-type.json", Some(branch)) else {
        return (full_name.to_string(), None, None);
    };
    let Ok(text) = github::decode_contents_payload(&meta) else {
        return (full_name.to_string(), None, None);
    };
    let Ok(doc) = serde_json::from_str::<Value>(&text) else {
        return (full_name.to_string(), None, None);
    };
    (
        full_name.to_string(),
        doc.get("type").and_then(|v| v.as_str()).map(str::to_string),
        doc.get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    )
}

const TYPE_META_FETCH_LIMIT: usize = 40;

fn cached_at_label() -> String {
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    Utc::now()
        .with_timezone(&offset)
        .format("%m月%d日 %H:%M")
        .to_string()
}

pub fn check_file_json(repo_root: &std::path::Path, repo: &str, path: &str) -> Value {
    if let Err(v) = require_known_repo(repo_root, repo, "repo and path required") {
        return v;
    }
    let path = path.trim();
    if path.is_empty() {
        return json!({ "error": "repo and path required", "_status": 400 });
    }
    let (owner, repo_name) = match repo.trim().split_once('/') {
        Some(p) => p,
        None => return json!({ "error": "repo and path required", "_status": 400 }),
    };
    match github::get_contents(owner, repo_name, path, None) {
        Ok(_) => json!({ "exists": true }),
        Err(e) if e.status == Some(404) => json!({ "exists": false }),
        Err(e) => gh_err_value(e),
    }
}

pub fn repo_dirs_json(repo_root: &std::path::Path, repo: &str) -> Value {
    if let Err(v) = require_known_repo(repo_root, repo, "missing or invalid repo") {
        return v;
    }
    let (owner, repo_name) = match repo.trim().split_once('/') {
        Some(p) => p,
        None => return json!({ "error": "missing or invalid repo", "_status": 400 }),
    };
    match github::get_contents(owner, repo_name, "", None) {
        Ok(v) => match parse_repo_dirs(&v) {
            Ok(out) => out,
            Err(e) => json!({ "error": e, "_status": 500 }),
        },
        Err(e) => gh_err_value(e),
    }
}

pub fn repo_list_json(repo_root: &std::path::Path, force: bool) -> Value {
    let cache_path = repo_list_cache_path(repo_root);
    if !force && cache_path.is_file() {
        if let Ok(text) = fs::read_to_string(&cache_path) {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                return v;
            }
        }
    }

    let mut all_repos = Vec::new();
    let mut page = 1u32;
    loop {
        let batch = match github::list_user_repos_page(page) {
            Ok(Value::Array(items)) => items,
            Ok(_) => break,
            Err(e) => return gh_err_value(e),
        };
        if batch.is_empty() {
            break;
        }
        all_repos.extend(batch);
        page += 1;
        if page > 50 {
            break;
        }
    }

    let mut type_map: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
    let mut meta_fetch_count = 0usize;
    for repo in &all_repos {
        if meta_fetch_count >= TYPE_META_FETCH_LIMIT {
            break;
        }
        let full_name = repo.get("full_name").and_then(|v| v.as_str()).unwrap_or("");
        if full_name.is_empty() {
            continue;
        }
        let branch = repo
            .get("default_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main");
        let (name, ty, desc) = fetch_type_meta(full_name, branch);
        type_map.insert(name, (ty, desc));
        meta_fetch_count += 1;
    }

    let mut repos_out = Vec::new();
    for repo in all_repos {
        let full_name = repo.get("full_name").and_then(|v| v.as_str()).unwrap_or("");
        let fallback_description = repo
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let (repo_type, description) = type_map
            .get(full_name)
            .cloned()
            .unwrap_or((None, fallback_description));
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
        let items = serde_json::json!([
          {"name": "docs", "type": "dir"},
          {"name": ".hidden", "type": "dir"},
          {"name": "readme.md", "type": "file"}
        ]);
        let v = parse_repo_dirs(&items).expect("parse");
        assert_eq!(v["dirs"], json!(["docs"]));
    }

    #[test]
    fn check_file_rejects_unknown_repo() {
        let dir = tempfile::tempdir().expect("tmp");
        let v = check_file_json(dir.path(), "unknown/foo", "a.md");
        assert!(v.get("error").is_some());
    }

    #[test]
    fn type_meta_fetch_limit_is_reasonable() {
        assert!(TYPE_META_FETCH_LIMIT > 0);
        assert!(TYPE_META_FETCH_LIMIT <= 100);
    }
}
