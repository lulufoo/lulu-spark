//! Business-side expansion of the Host file-tool path fence at Binding Set.
//! The Agent loop only stores the resulting path lists.

use crate::config::paths;
use crate::services::path_fence::{stored_path, PathFence};
use crate::mcp_host::registry::SEEDED_BUSINESS_KEY;
use crate::services::sediment_kb;

/// Expand fence lists for a business key. Unknown keys yield `None`.
pub fn expand_for_business_key(key: &str) -> Option<PathFence> {
    if key != SEEDED_BUSINESS_KEY {
        return None;
    }
    let wb = stored_path(paths::spark_root().ok()?);
    let data = stored_path(paths::runtime_data_dir());
    let cache = stored_path(paths::cache_dir().ok()?);
    let knowledge = stored_path(paths::knowledge_root().ok()?);

    // SPARK_DATA_DIR is closed to the LLM: it reaches notes and knowledge only
    // through `get_note_file` / `get_knowledge_file` copies. Deny wins over any
    // allow or staged grant, so this also closes write.
    let read_allow = vec![data.clone()];
    let mut read_deny = vec![data, wb.join(".git")];

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
                read_deny.push(dir.join(".git"));
            }
        }
    }

    Some(PathFence {
        read_allow,
        read_deny,
        write_allow: vec![],
        write_deny: vec![],
        scratch_parent: Some(cache.join("agent-workspace")),
    })
}

#[cfg(test)]
#[path = "../unit-tests/services/spark_path_fence.rs"]
mod tests;
