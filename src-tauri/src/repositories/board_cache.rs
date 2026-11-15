//! Type-independent Board source cache under the active runtime cache root.
//!
//! Callers validate Board URLs and provide an opaque credential scope. This
//! repository never reads credentials, calls Board, or interprets source text.

use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use chrono::{DateTime, SecondsFormat, Utc};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::settings;

use super::atomic_json;

mod files;

pub const SCHEMA_VERSION: u32 = 1;
/// A bounded, rebuildable cache. Oldest written entries are evicted first.
/// A single entry larger than this budget is left to the remote-read caller.
pub const DEFAULT_CAPACITY_BYTES: u64 = 64 * 1024 * 1024;

// All handles share this lock so a failed read cannot delete a replacement
// written by another handle. No network or credential IO runs under this lock.
static CACHE_IO: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoardCacheRecord {
    pub schema_version: u32,
    pub url: String,
    pub diagram_id: String,
    pub version: u32,
    pub bmd: String,
    pub title: String,
    pub credential_scope: String,
    pub cached_at: String,
}

impl BoardCacheRecord {
    pub fn new(
        url: String,
        diagram_id: String,
        version: u32,
        bmd: String,
        title: String,
        credential_scope: String,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            url,
            diagram_id,
            version,
            bmd,
            title,
            credential_scope,
            cached_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }

    fn valid(&self) -> bool {
        self.schema_version == SCHEMA_VERSION
            && !self.url.trim().is_empty()
            && valid_diagram_id(&self.diagram_id)
            && self.version > 0
            && !self.credential_scope.trim().is_empty()
            && DateTime::parse_from_rfc3339(&self.cached_at).is_ok()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreOutcome {
    Stored,
    SkippedTooLarge,
}

#[derive(Clone, Debug)]
pub struct BoardCache {
    directory: PathBuf,
    capacity_bytes: u64,
}

impl BoardCache {
    /// Use the installed runtime plane, including its test-sandbox isolation.
    pub fn runtime() -> Result<Self, String> {
        let root = settings::runtime_cache_dir().map_err(|e| format!("{e:?}"))?;
        Ok(Self::new(root))
    }

    /// Construct from a runtime cache root, not from its `board` subdirectory.
    pub fn new(runtime_cache_root: impl AsRef<Path>) -> Self {
        Self::with_capacity(runtime_cache_root, DEFAULT_CAPACITY_BYTES)
    }

    /// An explicit budget allows callers to use the same cache in smaller stores.
    pub fn with_capacity(runtime_cache_root: impl AsRef<Path>, capacity_bytes: u64) -> Self {
        Self {
            directory: runtime_cache_root.as_ref().join("board"),
            capacity_bytes,
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn path(&self, url: &str) -> PathBuf {
        self.directory.join(format!("{}.json", cache_key(url)))
    }

    /// A scope mismatch is a miss. It never returns another credential's source.
    /// Malformed records are deleted before returning a miss; deletion errors
    /// return no content and can be treated as cache failures by the caller.
    pub fn read(
        &self,
        url: &str,
        credential_scope: &str,
    ) -> Result<Option<BoardCacheRecord>, String> {
        let _guard = io_lock()?;
        let path = self.path(url);
        let Some(record) = self.read_record(&path)? else {
            return Ok(None);
        };
        if record.url != url {
            remove_file(&path)?;
            return Ok(None);
        }
        Ok((record.credential_scope == credential_scope).then_some(record))
    }

    /// This stores a caller-selected revision; request generations, not version
    /// ordering, decide whether a remote result is current in the service layer.
    pub fn store(&self, record: &BoardCacheRecord) -> Result<StoreOutcome, String> {
        if !record.valid() {
            return Err("invalid Board cache record".into());
        }
        let value = serde_json::to_value(record).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > self.capacity_bytes {
            return Ok(StoreOutcome::SkippedTooLarge);
        }
        let _guard = io_lock()?;
        self.remove_interrupted_writes()?;
        let path = self.path(&record.url);
        if let Err(error) = atomic_json::write_json(&path, &value) {
            let _ = fs::remove_file(path.with_extension("json.tmp"));
            return Err(error);
        }
        self.evict_oldest(&path)?;
        Ok(StoreOutcome::Stored)
    }

    pub fn remove_url(&self, url: &str) -> Result<(), String> {
        let _guard = io_lock()?;
        remove_file(&self.path(url))
    }

    /// Remove all URL aliases of this diagram within the specified scope.
    pub fn remove_diagram(
        &self,
        diagram_id: &str,
        credential_scope: &str,
    ) -> Result<usize, String> {
        let _guard = io_lock()?;
        self.remove_matching(|r| {
            r.diagram_id == diagram_id && r.credential_scope == credential_scope
        })
    }

    pub fn remove_scope(&self, credential_scope: &str) -> Result<usize, String> {
        let _guard = io_lock()?;
        self.remove_matching(|r| r.credential_scope == credential_scope)
    }

    /// Clear only the dedicated Board cache, including interrupted temp writes.
    pub fn clear(&self) -> Result<(), String> {
        let _guard = io_lock()?;
        match fs::remove_dir_all(&self.directory) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn read_record(&self, path: &Path) -> Result<Option<BoardCacheRecord>, String> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let mut bytes = Vec::new();
        file.take(self.capacity_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        // Discard files which cannot fit this store without reading unbounded data.
        if bytes.len() as u64 > self.capacity_bytes {
            remove_file(path)?;
            return Ok(None);
        }
        let value: Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => return discard(path),
        };
        match value.get("schema_version").and_then(Value::as_u64) {
            Some(version) if version != u64::from(SCHEMA_VERSION) && version > 0 => {
                return Ok(None);
            }
            Some(version) if version == u64::from(SCHEMA_VERSION) => {}
            _ => return discard(path),
        }
        let record = match serde_json::from_value::<BoardCacheRecord>(value) {
            Ok(record) if record.valid() => record,
            _ => return discard(path),
        };
        let expected_filename = format!("{}.json", cache_key(&record.url));
        if path.file_name().and_then(|s| s.to_str()) != Some(expected_filename.as_str()) {
            return discard(path);
        }
        Ok(Some(record))
    }

    fn remove_matching(
        &self,
        matches: impl Fn(&BoardCacheRecord) -> bool,
    ) -> Result<usize, String> {
        let mut removed = 0;
        for entry in self.entries()? {
            if let Some(record) = self.read_record(&entry.path)? {
                if matches(&record) {
                    remove_file(&entry.path)?;
                    removed += 1;
                }
            }
        }
        Ok(removed)
    }
}

/// The URL string's UTF-8 bytes are the entire key; no type or ID substitution.
pub fn cache_key(url: &str) -> String {
    format!("{:x}", Md5::digest(url.as_bytes()))
}

fn valid_diagram_id(id: &str) -> bool {
    id.strip_prefix("b_").is_some_and(|hex| {
        hex.len() == 8
            && hex
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    })
}

fn io_lock() -> Result<MutexGuard<'static, ()>, String> {
    CACHE_IO
        .lock()
        .map_err(|_| "Board cache IO lock poisoned".into())
}

fn remove_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn discard(path: &Path) -> Result<Option<BoardCacheRecord>, String> {
    remove_file(path)?;
    Ok(None)
}

#[cfg(test)]
#[path = "../unit-tests/repositories/board_cache.rs"]
mod tests;
