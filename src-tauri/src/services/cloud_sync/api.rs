//! What a sync run needs from the cloud, as a trait: the run is tested against a stand-in cloud,
//! and [`CloudSync`] is the real one.

use super::client::CloudSync;
use super::error::CloudSyncError;
use super::types::CloudDoc;

/// The cloud's documents, by document id. The methods are those of [`CloudSync`] with the same
/// names; see there for what each one does and returns.
pub trait CloudDocs {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError>;

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError>;

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError>;

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError>;
}

impl CloudDocs for CloudSync {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        CloudSync::list_documents(self)
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        CloudSync::upload(self, document_id, package, version)
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        CloudSync::read(self, document_id)
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        CloudSync::delete(self, document_id)
    }
}

/// Removes the cloud copy of a document that is being deleted on this machine.
///
/// A user who is not signed in has no cloud copy to remove, and an id that cannot name a cloud
/// object (see `upload`) was never uploaded: both are `Ok`, and the caller goes on with the local
/// delete. Every other failure is returned, because the copy may still be there: the caller must
/// then keep the document, or the next sync would bring it back.
pub fn delete_cloud_copy(cloud: &dyn CloudDocs, document_id: &str) -> Result<(), CloudSyncError> {
    match cloud.delete(document_id) {
        Err(CloudSyncError::NotSignedIn | CloudSyncError::InvalidInput(_)) => Ok(()),
        other => other,
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync_api.rs"]
mod tests;
