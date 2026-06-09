//! Shared GitHub URL parsing for blob, tree, and destination directory links.

pub fn parse_github_blob(url: &str) -> Option<(String, String, String, String)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 5 || parts[2] != "blob" {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[3].to_string(),
        parts[4..].join("/"),
    ))
}

pub fn parse_github_tree(url: &str) -> Option<(String, String, String, String)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 5 || parts[2] != "tree" {
        return None;
    }
    Some((
        parts[0].to_string(),
        parts[1].to_string(),
        parts[3].to_string(),
        parts[4..].join("/").trim_end_matches('/').to_string(),
    ))
}

/// Last path segment looks like a file (e.g. `rules_sync.json`, `design.md`).
fn path_ends_with_file_segment(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let Some(ext) = name.rsplit_once('.').map(|(_, ext)| ext) else {
        return false;
    };
    !ext.is_empty()
        && ext.len() <= 16
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
        && ext.chars().any(|c| c.is_alphabetic())
}

/// Strip `tree/{ref}/…` or `blob/{ref}/…` to a repo-relative path.
/// Returns `(path, is_blob_link)`.
fn strip_github_tree_or_blob_prefix(tail: &str) -> Option<(String, bool)> {
    let (rest, is_blob) = if let Some(r) = tail.strip_prefix("tree/") {
        (r, false)
    } else if let Some(r) = tail.strip_prefix("blob/") {
        (r, true)
    } else {
        return None;
    };
    let path = if let Some(idx) = rest.find('/') {
        rest[idx + 1..].to_string()
    } else {
        String::new()
    };
    Some((path, is_blob))
}

pub fn parse_github_dst(url: &str) -> Option<(String, String, String)> {
    let rest = url.strip_prefix("https://github.com/")?;
    let mut parts: Vec<&str> = rest.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    parts.drain(0..2);
    let tail = parts.join("/");

    let dir = match strip_github_tree_or_blob_prefix(&tail) {
        Some((path, is_blob)) => {
            if is_blob && !path.is_empty() && path_ends_with_file_segment(&path) {
                // blob 文件链接：目标为其所在目录
                path.rfind('/')
                    .map(|i| path[..i].to_string())
                    .unwrap_or_default()
            } else {
                path
            }
        }
        None => tail,
    };
    Some((owner, repo, dir.trim_matches('/').to_string()))
}

#[cfg(test)]
#[path = "../unit-tests/services/github_url.rs"]
mod tests;
