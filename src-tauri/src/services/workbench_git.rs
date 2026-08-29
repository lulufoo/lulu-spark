//! Workbench store git commit / pull / revert (`settings.workbench_root`).

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::config::paths;
use crate::integrations::git::{self, GitError};

fn workbench_root() -> Result<PathBuf, Value> {
    paths::workbench_root().map_err(|e| json!({ "error": format!("{e:?}") }))
}

fn not_a_git_repo(workbench: &std::path::Path) -> String {
    format!(
        "workbench_root is not a git repository: {}",
        workbench.display()
    )
}

pub fn workbench_git_commit(payload: &Value) -> Value {
    let workbench = match workbench_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !workbench.join(".git").exists() {
        return json!({ "error": not_a_git_repo(&workbench) });
    }
    let message = payload
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let message = if message.is_empty() {
        "update: edit via viewer"
    } else {
        message
    };
    if let Some(files) = payload.get("files").and_then(|v| v.as_array()) {
        let paths: Vec<String> = files
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
        if let Err(e) = git::add_paths(&workbench, &paths) {
            return git_error_value(&e, None);
        }
    } else if let Err(e) = git::add_all(&workbench) {
        return git_error_value(&e, None);
    }

    let (nothing, commit_out) = match git::commit(&workbench, message) {
        Ok(v) => v,
        Err(e) => return git_error_value(&e, None),
    };

    if nothing {
        match git::push(&workbench) {
            Ok(o) if o.success => {
                return json!({ "ok": true, "info": "nothing to commit" });
            }
            Ok(o) => {
                return json!({
                    "error": "git push failed",
                    "stderr": o.stderr
                });
            }
            Err(e) => return git_error_value(&e, None),
        }
    }

    if let Err(e) = git::pull_rebase_in_repo(&workbench) {
        return git_error_value(&e, None);
    }
    match git::push(&workbench) {
        Ok(o) if o.success => json!({ "ok": true, "info": commit_out }),
        Ok(o) => json!({ "error": "git push failed", "stderr": o.stderr }),
        Err(e) => git_error_value(&e, None),
    }
}

pub fn workbench_git_pull(_payload: &Value) -> Value {
    let workbench = match workbench_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !workbench.join(".git").exists() {
        return json!({ "error": not_a_git_repo(&workbench) });
    }
    match git::pull_rebase(&workbench) {
        Ok(o) if o.success => json!({ "ok": true, "info": o.stdout.trim() }),
        Ok(o) => json!({
            "error": "git pull --rebase failed",
            "stderr": o.stderr,
            "stdout": o.stdout
        }),
        Err(e) => git_error_value(&e, None),
    }
}

pub fn workbench_git_revert(payload: &Value) -> Value {
    let workbench = match workbench_root() {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !workbench.join(".git").exists() {
        return json!({ "error": not_a_git_repo(&workbench), "_status": 500 });
    }
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
    let file_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("").trim();

    if !path.is_empty() {
        if file_type == "new" {
            let target = workbench.join(path);
            if target.is_dir() {
                let _ = std::fs::remove_dir_all(&target);
            } else if target.exists() {
                let _ = std::fs::remove_file(&target);
            } else {
                let _ = git::exec(&workbench, &["clean", "-fx", "--", path]);
            }
        } else {
            let out = match git::checkout_paths(&workbench, &[path]) {
                Ok(o) => o,
                Err(e) => {
                    return json!({ "error": e.message, "_status": 500 });
                }
            };
            if !out.success {
                return json!({
                    "error": format!("revert failed: {}", out.stderr.trim()),
                    "_status": 500
                });
            }
        }
    } else {
        let r1 = if git::has_unmerged(&workbench).unwrap_or(false) {
            git::exec(&workbench, &["reset", "--hard", "HEAD"]).unwrap_or_else(|e| git::GitOutput {
                stdout: String::new(),
                stderr: e.message,
                success: false,
            })
        } else {
            git::checkout_paths(&workbench, &["."]).unwrap_or_else(|e| git::GitOutput {
                stdout: String::new(),
                stderr: e.message,
                success: false,
            })
        };
        if !r1.success {
            return json!({
                "error": format!("revert all failed: {}", r1.stderr.trim()),
                "_status": 500
            });
        }
        let _ = git::clean_force(&workbench, &["-fd"]);
    }
    json!({ "ok": true })
}

fn git_error_value(err: &GitError, step: Option<&str>) -> Value {
    let mut v = git::git_error_json(err, step);
    v["_status"] = json!(500);
    v
}

#[cfg(test)]
#[path = "../unit-tests/services/workbench_git.rs"]
mod tests;
