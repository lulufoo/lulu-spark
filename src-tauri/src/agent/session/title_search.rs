//! Search chats by title, across every chat on disk (not only the Home list's recent days).

use serde_json::Value;

use super::catalog;
use super::store::catalog_file_path;

/// Most hits one search may return, whatever the caller asks for.
const MAX_HITS: usize = 100;

pub fn search_session_titles(query: &str, limit: usize) -> Result<Vec<Value>, String> {
    catalog::search_titles(&catalog_file_path()?, query, limit.min(MAX_HITS))
}
