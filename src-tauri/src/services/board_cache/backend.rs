use crate::config::vault::{self, BoardCredentialSnapshot};
use crate::repositories::board_cache::{BoardCache, BoardCacheRecord, StoreOutcome};
use crate::services::board_sketch::{fetch_document, DocumentFetch};

use super::{CredentialStore, Fetcher, SourceCache};

pub(super) struct VaultCredentials;
impl CredentialStore for VaultCredentials {
    fn get(&self) -> Result<Option<BoardCredentialSnapshot>, String> {
        vault::get_board_credentials().map_err(|e| e.to_string())
    }
    fn set(&self, value: &str) -> Result<(), String> {
        vault::set_board_pat(value).map_err(|e| e.to_string())
    }
    fn delete(&self) -> Result<(), String> {
        vault::delete_board_pat().map_err(|e| e.to_string())
    }
    fn revoke(&self, scope: &str) -> Result<bool, String> {
        vault::revoke_board_credentials(scope).map_err(|e| e.to_string())
    }
    fn deny(&self, scope: &str, id: &str, denied: bool) -> Result<bool, String> {
        vault::set_board_diagram_denied(scope, id, denied).map_err(|e| e.to_string())
    }
}

// Resolve per operation: startup installs the runtime plane after app setup.
pub(super) struct RuntimeCache;
impl SourceCache for RuntimeCache {
    fn read(&self, url: &str, scope: &str) -> Result<Option<BoardCacheRecord>, String> {
        BoardCache::runtime()?.read(url, scope)
    }
    fn store(&self, value: &BoardCacheRecord) -> Result<StoreOutcome, String> {
        BoardCache::runtime()?.store(value)
    }
    fn remove_url(&self, url: &str) -> Result<(), String> { BoardCache::runtime()?.remove_url(url) }
    fn remove_diagram(&self, id: &str, scope: &str) -> Result<usize, String> {
        BoardCache::runtime()?.remove_diagram(id, scope)
    }
    fn remove_scope(&self, scope: &str) -> Result<usize, String> {
        BoardCache::runtime()?.remove_scope(scope)
    }
    fn clear(&self) -> Result<(), String> { BoardCache::runtime()?.clear() }
}

impl SourceCache for BoardCache {
    fn read(&self, url: &str, scope: &str) -> Result<Option<BoardCacheRecord>, String> {
        BoardCache::read(self, url, scope)
    }
    fn store(&self, value: &BoardCacheRecord) -> Result<StoreOutcome, String> {
        BoardCache::store(self, value)
    }
    fn remove_url(&self, url: &str) -> Result<(), String> { BoardCache::remove_url(self, url) }
    fn remove_diagram(&self, id: &str, scope: &str) -> Result<usize, String> {
        BoardCache::remove_diagram(self, id, scope)
    }
    fn remove_scope(&self, scope: &str) -> Result<usize, String> {
        BoardCache::remove_scope(self, scope)
    }
    fn clear(&self) -> Result<(), String> { BoardCache::clear(self) }
}

pub(super) struct HttpFetcher;
impl Fetcher for HttpFetcher {
    fn fetch(&self, credential: &str, id: &str) -> DocumentFetch {
        fetch_document(credential, id)
    }
}
