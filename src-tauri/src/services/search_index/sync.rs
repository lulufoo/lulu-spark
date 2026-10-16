//! App-internal write paths → keep the index in step without a full rebuild.
//!
//! Scope (2026-09-05 /converge): create / save / delete done by the App itself. Moving
//! a note needs no sync: chunks carry no path. Files edited outside the App still rely
//! on the startup / manual rebuild.

use super::chunk_rows::replace_doc_chunks;
use super::collect_docs::workspace_doc_chunks;
use super::query::cache_dir_or_err;
use super::store::{index_exists, open};

/// Re-chunk each document `doc_id` of `workspace_id` and replace its rows; a document whose row
/// is gone has its rows dropped. No-op when no index file exists.
pub fn sync_workspace_docs(workspace_id: &str, doc_ids: &[&str]) -> Result<usize, String> {
    let cache = cache_dir_or_err()?;
    if !index_exists(&cache) {
        return Ok(0);
    }
    let conn = open(&cache)?;
    for doc_id in doc_ids {
        let chunks = workspace_doc_chunks(workspace_id, doc_id)?;
        replace_doc_chunks(&conn, doc_id, &chunks)?;
    }
    Ok(doc_ids.len())
}

/// Same as [`sync_workspace_docs`] but never fails the caller's write response.
pub fn sync_workspace_docs_best_effort(workspace_id: &str, doc_ids: &[&str]) {
    if let Err(e) = sync_workspace_docs(workspace_id, doc_ids) {
        eprintln!("search index sync skipped: {e}");
    }
}
