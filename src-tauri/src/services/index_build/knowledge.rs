//! Knowledge search index (registered repos → FTS5).

use std::path::Path;

use crate::config::paths;
use crate::services::search_index;

/// `force_repo = Some("owner/name")` scopes both upsert and ghost deletion to one repo.
pub fn rebuild(repo_root: &Path, force_repo: Option<&str>) -> Result<String, String> {
    let cache = paths::cache_dir().map_err(|e| format!("{e:?}"))?;
    let chunks = search_index::collect_knowledge(repo_root, force_repo);
    let stats = search_index::rebuild(&cache, &chunks, Some("knowledge"), force_repo)?;
    Ok(format!(
        "Indexed {} chunks ({} unchanged, {} removed)",
        stats.upserted + stats.skipped,
        stats.skipped,
        stats.deleted
    ))
}
