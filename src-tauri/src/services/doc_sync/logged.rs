//! Writing the calls of a run to the log as they go through, not when the run is over: what was
//! sent, how big, how long it took. A run of a thousand documents can stop at any one, and the
//! log must show how far it got and how fast.
//!
//! Only what went through is written here. What went wrong is returned to the run, which knows
//! the document and the operation, and it is written when the run ends (see `journal::lines_of`).
//! The lines are handed to `live` one by one: the app writes them to the log, a test collects
//! them. A line holds ids, counts and times, never a link or a token.

use std::time::Instant;

use super::journal::{self, Line};
use crate::services::cloud_sync::{CloudDoc, CloudDocs, CloudSyncError};

/// `cloud`, with a line for each list, upload and read that went through.
pub(super) struct Logged<'a> {
    cloud: &'a dyn CloudDocs,
    live: &'a dyn Fn(Line),
}

impl<'a> Logged<'a> {
    pub(super) fn new(cloud: &'a dyn CloudDocs, live: &'a dyn Fn(Line)) -> Self {
        Self { cloud, live }
    }
}

fn ms_since(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX)
}

impl CloudDocs for Logged<'_> {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        let start = Instant::now();
        let documents = self.cloud.list_documents()?;
        (self.live)(journal::line_listed(documents.len(), ms_since(start)));
        Ok(documents)
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        let start = Instant::now();
        self.cloud.upload(document_id, package, version)?;
        (self.live)(journal::line_uploaded(document_id, version, package.len(), ms_since(start)));
        Ok(())
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        let start = Instant::now();
        let package = self.cloud.read(document_id)?;
        (self.live)(journal::line_downloaded(document_id, package.len(), ms_since(start)));
        Ok(package)
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        self.cloud.delete(document_id)
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_logged.rs"]
mod tests;
