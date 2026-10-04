//! GitHub read-only helpers via REST (`integrations/github.rs`).

use serde_json::{json, Value};

use crate::integrations::github::{self, GithubError};
use crate::services::spark_read::get_topics;

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

#[cfg(test)]
#[path = "../unit-tests/integrations/gh_read.rs"]
mod tests;
