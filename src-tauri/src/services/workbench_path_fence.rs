//! Business-side expansion of the Host file-tool path fence at Binding Set.
//! The Agent loop only stores the resulting path lists.

use crate::config::paths;
use crate::services::agent::path_fence::{stored_path, PathFence};
use crate::services::mcp_host::registry::SEEDED_BUSINESS_KEY;
use crate::services::sediment_kb;

/// Expand fence lists for a business key. Unknown keys yield `None`.
pub fn expand_for_business_key(key: &str) -> Option<PathFence> {
    if key != SEEDED_BUSINESS_KEY {
        return None;
    }
    let wb = stored_path(paths::workbench_root().ok()?);
    let cache = stored_path(paths::cache_dir().ok()?);
    let knowledge = stored_path(paths::knowledge_root().ok()?);

    let mut read_allow = vec![wb.clone()];
    let mut read_deny = vec![wb.join(".git")];

    let repos_file = paths::sediment_kb_repos_path().ok();
    if repos_file.as_ref().is_some_and(|path| path.is_file()) {
        if let Ok(repos) = sediment_kb::load_repos() {
            for repo in repos.repos {
                let Some(name) = repo.full_name.split('/').next_back() else {
                    continue;
                };
                if name.is_empty() {
                    continue;
                }
                let dir = stored_path(knowledge.join(name));
                read_allow.push(dir.clone());
                read_deny.push(dir.join(".git"));
            }
        }
    }

    Some(PathFence {
        read_allow,
        read_deny,
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: Some(cache.join("agent-scratch")),
    })
}

#[cfg(test)]
#[path = "../unit-tests/services/workbench_path_fence.rs"]
mod tests;
