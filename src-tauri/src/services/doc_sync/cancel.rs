//! Cancelling a run without the run knowing: the cloud it is given refuses the next transfer.
//!
//! The push and pull loops already stop at an error that would hit every document and report the
//! documents they did not get to as `not_tried`. A cancelled run is such a stop, so it needs no
//! second path through the loops: once the user asked, every upload and every read answers
//! [`CloudSyncError::Cancelled`].
//!
//! The document that is being transferred when the user asks is finished, not cut: a transfer is
//! one blocking call, and a half-written package would be worse than a few seconds of waiting.

use super::gate::SyncRun;
use crate::services::cloud_sync::{CloudDoc, CloudDocs, CloudSyncError};

/// `cloud`, for as long as `run` has not been cancelled.
///
/// The list is not held back: it is one short call, and a run cancelled while it goes on is
/// stopped at the first transfer after it. Deleting is not part of a run.
pub(super) struct Cancellable<'a> {
    cloud: &'a dyn CloudDocs,
    run: &'a SyncRun,
}

impl<'a> Cancellable<'a> {
    pub(super) fn new(cloud: &'a dyn CloudDocs, run: &'a SyncRun) -> Self {
        Self { cloud, run }
    }

    fn ensure_going(&self) -> Result<(), CloudSyncError> {
        if self.run.is_cancelled() {
            Err(CloudSyncError::Cancelled)
        } else {
            Ok(())
        }
    }
}

impl CloudDocs for Cancellable<'_> {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        self.cloud.list_documents()
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        self.ensure_going()?;
        self.cloud.upload(document_id, package, version)
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.ensure_going()?;
        self.cloud.read(document_id)
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        self.cloud.delete(document_id)
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_cancel.rs"]
mod tests;
