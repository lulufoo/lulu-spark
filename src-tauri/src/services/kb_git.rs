//! Knowledge-base per-repo git operations under `{kb_root}/{repo_name}`.

use std::path::PathBuf;

use serde_json::{json, Value};

use crate::config::paths;
use crate::integrations::git::{self, GitError};

fn kb_repo_dir(repo: &str) -> Result<PathBuf, Value> {
    let repo = repo.trim();
    if repo.is_empty() || !repo.contains('/') {
        return Err(json!({ "error": "invalid repo format", "_status": 400 }));
    }
    let repo_name = repo.split('/').next_back().unwrap_or("");
    let kb_root = paths::knowledge_corpus_root().map_err(|e| json!({ "error": format!("{e:?}") }))?;
    let local = kb_root.join(repo_name);
    if !local.is_dir() {
        return Err(json!({
            "error": format!("repo not cloned locally: {repo_name}"),
            "_status": 404
        }));
    }
    Ok(local)
}

pub fn kb_git_commit(payload: &Value) -> Value {
    let repo = payload.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let local = match kb_repo_dir(repo) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let message = payload
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let message = if message.is_empty() {
        "chore: update via viewer"
    } else {
        message
    };

    if git::has_unmerged(&local).unwrap_or(false) {
        return git::git_error_json(
            &GitError {
                message: "仓库存在未解决的合并冲突，请手动修复后重试".into(),
                stderr: None,
                stdout: None,
            },
            Some("pre-check"),
        );
    }

    let (stash_empty, stash_out) = match git::stash_save(&local) {
        Ok(v) => v,
        Err(e) => return step_err(&e, "stash"),
    };
    if !stash_out.success && !stash_empty {
        return step_err(
            &GitError {
                message: "git stash failed".into(),
                stderr: Some(stash_out.stderr),
                stdout: Some(stash_out.stdout),
            },
            "stash",
        );
    }

    if let Err(e) = git::pull_rebase_in_repo(&local) {
        if !stash_empty {
            let _ = git::stash_pop(&local);
        }
        return step_err(&e, "pull");
    }

    if !stash_empty {
        let pop = match git::stash_pop(&local) {
            Ok(o) => o,
            Err(e) => return step_err(&e, "stash_pop"),
        };
        if !pop.success {
            return step_err(
                &GitError {
                    message: "git stash pop failed".into(),
                    stderr: Some(pop.stderr),
                    stdout: Some(pop.stdout),
                },
                "stash_pop",
            );
        }
    }

    if let Err(e) = git::add_all(&local) {
        return step_err(&e, "add");
    }
    let ann = local.join(".knowledge_annotations");
    if ann.is_dir() {
        let _ = git::exec(&local, &["add", "--force", ".knowledge_annotations/"]);
    }

    let (nothing, commit_out) = match git::commit(&local, message) {
        Ok(v) => v,
        Err(e) => return step_err(&e, "commit"),
    };

    match git::push(&local) {
        Ok(o) if o.success => {
            let info = if nothing {
                "nothing to commit; pushed"
            } else {
                commit_out.as_str()
            };
            json!({ "ok": true, "info": info })
        }
        Ok(o) => json!({
            "error": "git push failed",
            "step": "push",
            "stderr": o.stderr
        }),
        Err(e) => step_err(&e, "push"),
    }
}

pub fn kb_git_revert(payload: &Value) -> Value {
    let repo = payload.get("repo").and_then(|v| v.as_str()).unwrap_or("");
    let local = match kb_repo_dir(repo) {
        Ok(p) => p,
        Err(v) => return v,
    };
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("").trim();
    let file_type = payload.get("type").and_then(|v| v.as_str()).unwrap_or("").trim();

    if !path.is_empty() {
        let is_ann = path.starts_with(".knowledge_annotations/");
        if file_type == "new" || is_ann {
            let target = local.join(path);
            if target.is_dir() {
                let _ = std::fs::remove_dir_all(&target);
            } else if target.exists() {
                let _ = std::fs::remove_file(&target);
            } else {
                let _ = git::exec(&local, &["clean", "-fx", "--", path]);
            }
        } else {
            let out = match git::checkout_paths(&local, &[path]) {
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
        let r1 = git::checkout_paths(&local, &["."]).unwrap_or_else(|e| git::GitOutput {
            stdout: String::new(),
            stderr: e.message,
            success: false,
        });
        if !r1.success {
            return json!({
                "error": format!("revert all failed: {}", r1.stderr.trim()),
                "_status": 500
            });
        }
        let _ = git::clean_force(&local, &["-fdx", ".knowledge_annotations/"]);
        let _ = git::clean_force(&local, &["-fd"]);
    }
    json!({ "ok": true })
}

fn step_err(err: &GitError, step: &str) -> Value {
    let mut v = git::git_error_json(err, Some(step));
    v["_status"] = json!(500);
    v
}

#[cfg(test)]
#[path = "../unit-tests/services/kb_git.rs"]
mod tests;
