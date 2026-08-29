use std::path::Path;

/// Personal GitHub home (`https://github.com/{owner}`) + workbench clone dir name → blob base for file links.
pub fn workbench_github_blob_base(github_user_url: &str, workbench_root: &Path) -> String {
    let trimmed = github_user_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    let repo = workbench_root
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("lulu-workbench-knowledge");
    format!("{trimmed}/{repo}/blob/main")
}

fn workbench_git_origin_url(workbench_root: &Path) -> Option<String> {
    crate::integrations::git::origin_url(workbench_root)
}

/// `https://github.com/{owner}` from `git remote get-url origin` when workbench root is a git repo.
pub fn infer_github_user_url_from_workbench_root(workbench_root: &Path) -> Option<String> {
    github_user_home_from_remote_url(&workbench_git_origin_url(workbench_root)?)
}

/// Profile + repo URL from one origin read.
pub fn infer_workbench_github_from_root(workbench_root: &Path) -> (Option<String>, Option<String>) {
    match workbench_git_origin_url(workbench_root) {
        Some(origin) => (
            github_user_home_from_remote_url(&origin),
            github_repo_url_from_remote_url(&origin),
        ),
        None => (None, None),
    }
}

/// Parse owner home URL from a GitHub remote (HTTPS or SSH).
pub fn github_user_home_from_remote_url(remote: &str) -> Option<String> {
    let s = remote.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
    {
        let owner = rest.split('/').next()?.trim();
        if owner.is_empty() {
            return None;
        }
        return Some(format!("https://github.com/{owner}"));
    }
    if let Some(rest) = s.strip_prefix("git@github.com:") {
        let owner = rest.split('/').next()?.trim();
        if owner.is_empty() {
            return None;
        }
        return Some(format!("https://github.com/{owner}"));
    }
    if let Some(rest) = s.strip_prefix("ssh://git@github.com/") {
        let owner = rest.split('/').next()?.trim();
        if owner.is_empty() {
            return None;
        }
        return Some(format!("https://github.com/{owner}"));
    }
    None
}

/// Parse `https://github.com/{owner}/{repo}` from a GitHub remote (HTTPS or SSH).
pub fn github_repo_url_from_remote_url(remote: &str) -> Option<String> {
    let s = remote.trim();
    if s.is_empty() {
        return None;
    }
    let rest = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.strip_prefix("git@github.com:"))
        .or_else(|| s.strip_prefix("ssh://git@github.com/"))?;
    let mut parts = rest.split('/');
    let owner = parts.next()?.trim();
    let repo = parts
        .next()?
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("https://github.com/{owner}/{repo}"))
}
