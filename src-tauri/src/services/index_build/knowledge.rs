//! Knowledge search index (registered repos → FTS5).

use std::collections::HashSet;
use std::path::Path;

use crate::config::paths;
use crate::services::knowledge::scan_registered_repos;
use crate::services::search_index;
use crate::services::spark_data::{self, knowledge_doc_ids_outside_repo};

/// `force_repo = Some("owner/name")` scopes both upsert and ghost deletion to one repo.
pub fn rebuild(repo_root: &Path, force_repo: Option<&str>) -> Result<String, String> {
    let cache = paths::cache_dir().map_err(|e| format!("{e:?}"))?;
    scan_registered_repos(repo_root, force_repo)?;
    let chunks = search_index::collect_knowledge(repo_root, force_repo)?;
    // A scoped rebuild must leave the other repos' chunks alone.
    let keep_docs = match force_repo {
        Some(full) => {
            let dir = full.split('/').next_back().unwrap_or(full);
            knowledge_doc_ids_outside_repo(&spark_data::open()?, dir)?
        }
        None => HashSet::new(),
    };
    let stats = search_index::rebuild(&cache, &chunks, Some("knowledge"), &keep_docs)?;
    Ok(format!(
        "Indexed {} chunks ({} unchanged, {} removed)",
        stats.upserted + stats.skipped,
        stats.skipped,
        stats.deleted
    ))
}

#[cfg(test)]
#[path = "../../unit-tests/services/index_build/knowledge.rs"]
mod tests;
