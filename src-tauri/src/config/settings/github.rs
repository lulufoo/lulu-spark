use std::path::Path;

/// Personal GitHub home (`https://github.com/{owner}`) + spark clone dir name → blob base for file links.
pub fn spark_github_blob_base(github_user_url: &str, spark_root: &Path) -> String {
    let trimmed = github_user_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    let repo = spark_root
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("lulu-workbench-knowledge");
    format!("{trimmed}/{repo}/blob/main")
}

fn spark_git_origin_url(spark_root: &Path) -> Option<String> {
    crate::integrations::git::origin_url(spark_root)
}

/// `https://github.com/{owner}` from `git remote get-url origin` when spark root is a git repo.
pub fn infer_github_user_url_from_spark_root(spark_root: &Path) -> Option<String> {
    github_user_home_from_remote_url(&spark_git_origin_url(spark_root)?)
}

/// Profile + repo URL from one origin read.
pub fn infer_spark_github_from_root(spark_root: &Path) -> (Option<String>, Option<String>) {
    match spark_git_origin_url(spark_root) {
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
