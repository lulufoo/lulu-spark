//! Knowledge repo scan. Knowledge documents are indexed with every other workspace's
//! (`spark::full_rebuild`), so this only refreshes the registered repos.

use std::path::Path;

use crate::services::knowledge::scan_registered_repos;

/// `force_repo = Some("folder")` rescans one topic only.
pub fn rebuild(repo_root: &Path, force_repo: Option<&str>) -> Result<String, String> {
    scan_registered_repos(repo_root, force_repo)?;
    Ok("Knowledge repos scanned (documents are indexed with their workspace)".to_string())
}

#[cfg(test)]
#[path = "../../unit-tests/services/index_build/knowledge.rs"]
mod tests;
