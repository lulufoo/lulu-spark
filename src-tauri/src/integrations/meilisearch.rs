//! Thin compatibility layer; search logic lives in `integrations::search`.

use std::path::Path;

use serde_json::Value;

use crate::integrations::search::default_backend;

pub fn search_json(repo_root: &Path, index_uid: &str, q: &str, limit: Option<u32>) -> Value {
    default_backend(repo_root).search(index_uid, q, limit)
}
