mod meili_admin;
mod meili_backend;

use std::path::Path;
use std::sync::Arc;

use serde_json::Value;

pub use meili_admin::MeiliAdminError;
pub use meili_backend::{
    build_knowledge_document, build_search_body, build_search_body_filtered, documents_path,
    map_search_result, parse_limit, search_path, MeiliBackend,
};

pub trait SearchBackend: Send + Sync {
    fn health(&self) -> bool;
    fn search(&self, index_uid: &str, q: &str, limit: Option<u32>) -> Value;
}

pub fn default_backend(repo_root: &Path) -> Arc<dyn SearchBackend> {
    Arc::new(MeiliBackend::new(repo_root))
}
