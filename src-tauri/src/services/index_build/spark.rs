//! Search index of every workspace's documents (`raw` / `digest` → FTS5).

use std::collections::HashSet;
use std::path::Path;

use crate::config::paths;
use crate::services::search_index;

pub fn full_rebuild(_repo_root: &Path) -> Result<String, String> {
    let cache = paths::cache_dir().map_err(|e| format!("{e:?}"))?;
    let chunks = search_index::collect_workspace_docs()?;
    let stats = search_index::rebuild(&cache, &chunks, None, &HashSet::new())?;
    if chunks.is_empty() {
        return Ok(format!(
            "No documents found. Removed {} stale chunks.",
            stats.deleted
        ));
    }
    Ok(format!(
        "Indexed {} chunks ({} unchanged, {} removed)",
        stats.upserted + stats.skipped,
        stats.skipped,
        stats.deleted
    ))
}
