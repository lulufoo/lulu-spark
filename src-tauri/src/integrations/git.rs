//! Single git implementation layer (git2 + scoped CLI for rebase/stash/checkout).

use std::path::Path;
use std::process::Command;

use git2::{Repository, Status, StatusOptions};
use serde_json::{json, Value};

#[derive(Debug)]
pub struct GitError {
    pub message: String,
    pub stderr: Option<String>,
    pub stdout: Option<String>,
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct GitOutput {
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub fn open_repo(path: &Path) -> Result<Repository, GitError> {
    Repository::open(path).map_err(|e| GitError {
        message: format!("open repo: {e}"),
        stderr: None,
        stdout: None,
    })
}

/// `origin` URL from git2 (no `git` on PATH required).
pub fn origin_url(repo: &Path) -> Option<String> {
    let repository = open_repo(repo).ok()?;
    let remote = repository.find_remote("origin").ok()?;
    let url = remote.url()?.trim();
    if url.is_empty() {
        None
    } else {
        Some(url.to_string())
    }
}

/// Run git CLI in `repo` (only subprocess site in this module).
pub(crate) fn exec(repo: &Path, args: &[&str]) -> Result<GitOutput, GitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .map_err(|e| GitError {
            message: format!("spawn git: {e}"),
            stderr: None,
            stdout: None,
        })?;
    Ok(GitOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    })
}

pub fn status_porcelain(repo: &Path) -> Result<String, GitError> {
    let repository = open_repo(repo)?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true);
    let statuses = repository.statuses(Some(&mut opts)).map_err(|e| GitError {
        message: format!("status: {e}"),
        stderr: None,
        stdout: None,
    })?;
    let mut lines = Vec::new();
    for entry in statuses.iter() {
        let st = entry.status();
        let path = entry.path().unwrap_or("");
        let x = if st.intersects(
            Status::INDEX_MODIFIED
                | Status::INDEX_NEW
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        ) {
            'M'
        } else {
            ' '
        };
        let y = if st.intersects(
            Status::WT_MODIFIED
                | Status::WT_NEW
                | Status::WT_DELETED
                | Status::WT_RENAMED
                | Status::WT_TYPECHANGE,
        ) {
            'M'
        } else if st.contains(Status::WT_NEW) {
            '?'
        } else {
            ' '
        };
        let xy = if st.is_conflicted() {
            "UU".to_string()
        } else if st.contains(Status::WT_RENAMED) || st.contains(Status::INDEX_RENAMED) {
            format!("{x}R")
        } else if st.contains(Status::WT_DELETED) || st.contains(Status::INDEX_DELETED) {
            format!("{x}D")
        } else if st.contains(Status::WT_NEW) && !st.intersects(Status::INDEX_NEW) {
            "??".to_string()
        } else {
            format!("{x}{y}")
        };
        lines.push(format!("{xy} {path}"));
    }
    Ok(lines.join("\n"))
}

pub fn ahead_count(repo: &Path) -> Result<i64, GitError> {
    let out = exec(repo, &["rev-list", "--count", "HEAD...@{u}"])?;
    if !out.success {
        return Ok(0);
    }
    Ok(out.stdout.trim().parse::<i64>().unwrap_or(0))
}

pub fn add_all(repo: &Path) -> Result<(), GitError> {
    let out = exec(repo, &["add", "-A"])?;
    if out.success {
        Ok(())
    } else {
        Err(GitError {
            message: "git add failed".into(),
            stderr: Some(out.stderr),
            stdout: Some(out.stdout),
        })
    }
}

pub fn add_paths(repo: &Path, paths: &[String]) -> Result<(), GitError> {
    let filtered: Vec<&str> = paths.iter().map(|s| s.as_str()).filter(|s| !s.is_empty()).collect();
    if filtered.is_empty() {
        return add_all(repo);
    }
    let mut args = vec!["add", "--"];
    args.extend(filtered);
    let out = exec(repo, &args)?;
    if out.success {
        Ok(())
    } else {
        Err(GitError {
            message: "git add failed".into(),
            stderr: Some(out.stderr),
            stdout: Some(out.stdout),
        })
    }
}

pub fn commit(repo: &Path, message: &str) -> Result<(bool /*nothing_to_commit*/, String), GitError> {
    let out = exec(repo, &["commit", "-m", message])?;
    let nothing = out.stdout.contains("nothing to commit") || out.stderr.contains("nothing to commit");
    if out.success || nothing {
        Ok((nothing, out.stdout.trim().to_string()))
    } else {
        Err(GitError {
            message: "git commit failed".into(),
            stderr: Some(out.stderr),
            stdout: Some(out.stdout),
        })
    }
}

pub fn pull_rebase(repo: &Path) -> Result<GitOutput, GitError> {
    exec(repo, &["pull", "--rebase"])
}

pub fn push(repo: &Path) -> Result<GitOutput, GitError> {
    exec(repo, &["push"])
}

pub fn has_unmerged(repo: &Path) -> Result<bool, GitError> {
    let out = exec(repo, &["ls-files", "--unmerged"])?;
    Ok(!out.stdout.trim().is_empty())
}

pub fn stash_save(repo: &Path) -> Result<(bool /*stash_was_empty*/, GitOutput), GitError> {
    let out = exec(repo, &["stash"])?;
    let empty = out.stdout.contains("No local changes to save");
    Ok((empty, out))
}

pub fn stash_pop(repo: &Path) -> Result<GitOutput, GitError> {
    exec(repo, &["stash", "pop"])
}

pub fn checkout_paths(repo: &Path, paths: &[&str]) -> Result<GitOutput, GitError> {
    let mut args = vec!["checkout", "--"];
    args.extend(paths.iter().copied());
    exec(repo, args.as_slice())
}

pub fn clean_force(repo: &Path, args: &[&str]) -> Result<GitOutput, GitError> {
    let mut cmd = vec!["clean"];
    cmd.extend(args.iter().copied());
    exec(repo, cmd.as_slice())
}

/// Clone `https://github.com/{owner}/{repo}.git` into `dest` (uses PAT when set).
pub fn clone_repo(repo: &str, dest: &Path) -> Result<GitOutput, GitError> {
    if dest.exists() {
        return Err(GitError {
            message: format!("destination exists: {}", dest.display()),
            stderr: None,
            stdout: None,
        });
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| GitError {
            message: e.to_string(),
            stderr: None,
            stdout: None,
        })?;
    }
    let mut url = format!("https://github.com/{repo}.git");
    if let Ok(Some(token)) = crate::config::secrets::get_secret(crate::config::secrets::KEY_GITHUB_TOKEN) {
        if !token.is_empty() {
            url = format!("https://x-access-token:{token}@github.com/{repo}.git");
        }
    }
    let parent = dest.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = dest
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| GitError {
            message: "invalid dest path".into(),
            stderr: None,
            stdout: None,
        })?;
    exec(parent, &["clone", &url, name])
}

pub fn pull_rebase_in_repo(repo: &Path) -> Result<(), GitError> {
    let out = pull_rebase(repo)?;
    if out.success {
        Ok(())
    } else {
        Err(GitError {
            message: "git pull --rebase failed".into(),
            stderr: Some(out.stderr),
            stdout: Some(out.stdout),
        })
    }
}

pub fn git_error_json(err: &GitError, step: Option<&str>) -> Value {
    let mut v = json!({
        "error": err.message,
        "stderr": err.stderr.as_deref().unwrap_or(""),
    });
    if let Some(s) = step {
        v["step"] = json!(s);
    }
    if let Some(ref o) = err.stdout {
        if !o.is_empty() {
            v["stdout"] = json!(o);
        }
    }
    v
}

#[cfg(test)]
#[path = "../unit-tests/integrations/git.rs"]
mod tests;
