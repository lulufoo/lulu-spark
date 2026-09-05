use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::config::roots::workbench_root_path;

pub fn get_draft(_repo_root: &Path, path: &str) -> Value {
    let decoded = urlencoding::decode(path).unwrap_or_else(|_| path.into());
    let common_path = decoded.trim();
    if common_path.is_empty() || common_path.contains("..") {
        return json!({ "error": "Invalid path" });
    }
    let target = match crate::config::paths::draft_path(common_path) {
        Ok(p) => p,
        Err(_) => return json!({ "error": "Invalid path" }),
    };
    let content = fs::read_to_string(&target).unwrap_or_default();
    json!({ "content": content })
}

pub fn categories_from_git_status(stdout: &str) -> Map<String, Value> {
    let mut categories: Map<String, Value> = Map::from_iter([
        ("new".to_string(), json!([])),
        ("modified".to_string(), json!([])),
        ("deleted".to_string(), json!([])),
        ("renamed".to_string(), json!([])),
        ("conflicted".to_string(), json!([])),
    ]);
    for line in stdout.lines() {
        let line = line.trim_end();
        if line.len() < 4 {
            continue;
        }
        let xy = &line[..2];
        let path_part = &line[3..];
        let x = xy.chars().next().unwrap_or(' ');
        let y = xy.chars().nth(1).unwrap_or(' ');
        let mut push = |key: &str, p: String| {
            categories
                .get_mut(key)
                .and_then(|v| v.as_array_mut())
                .expect("array")
                .push(json!(p));
        };
        if matches!(xy, "UU" | "AA" | "DD" | "AU" | "UA" | "DU" | "UD") {
            push("conflicted", path_part.trim().to_string());
        } else if x == 'R' || y == 'R' {
            if path_part.contains(" -> ") {
                let parts: Vec<&str> = path_part.splitn(2, " -> ").collect();
                if parts.len() == 2 {
                    push(
                        "renamed",
                        format!("{} → {}", parts[0].trim(), parts[1].trim()),
                    );
                } else {
                    push("renamed", path_part.trim().to_string());
                }
            } else {
                push("renamed", path_part.trim().to_string());
            }
        } else if x == 'D' || y == 'D' {
            push("deleted", path_part.trim().to_string());
        } else if x == 'A' || xy == "??" {
            push("new", path_part.trim().to_string());
        } else if x == 'M' || y == 'M' {
            push("modified", path_part.trim().to_string());
        } else if xy.trim().len() >= 1 {
            push("modified", path_part.trim().to_string());
        }
    }
    categories
}

pub fn get_status(repo_root: &Path) -> Value {
    let workbench = workbench_root_path(repo_root);
    let git_root = workbench.join(".git");
    if !git_root.exists() {
        let msg = format!(
            "workbench_root is not a git repository: {}",
            workbench.display()
        );
        return json!({ "error": msg });
    }
    let stdout = match crate::integrations::git::status_porcelain(&workbench) {
        Ok(s) => s,
        Err(e) => return json!({ "error": e.message }),
    };
    let mut categories = categories_from_git_status(&stdout);
    let total: usize = ["new", "modified", "deleted", "renamed", "conflicted"]
        .iter()
        .filter_map(|k| categories.get(*k).and_then(|v| v.as_array()))
        .map(|a| a.len())
        .sum();
    let ahead = crate::integrations::git::ahead_count(&workbench).unwrap_or(0);
    categories.insert("total".into(), json!(total));
    categories.insert("ahead".into(), json!(ahead));
    categories.insert(
        "workbench_root".into(),
        json!(workbench.to_string_lossy().to_string()),
    );
    Value::Object(categories)
}
