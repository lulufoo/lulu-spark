//! Spark notes search index (`raw` / `digest` → FTS5).

use std::path::Path;

use crate::config::paths;
use crate::services::search_index;

pub fn full_rebuild(repo_root: &Path) -> Result<String, String> {
    let cache = paths::cache_dir().map_err(|e| format!("{e:?}"))?;
    let chunks = search_index::collect_notes(repo_root);
    let stats = search_index::rebuild(&cache, &chunks, Some("notes"), None)?;
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
