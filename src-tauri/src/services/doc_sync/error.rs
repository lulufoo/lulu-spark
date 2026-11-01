//! Why one step of a sync run failed.

use std::fmt;

use crate::services::cloud_sync::CloudSyncError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocSyncError {
    /// The local database could not be read.
    Database(String),
    /// The files of the document could not be packed: a file the row names is missing, a name is
    /// unsafe, and so on. The document is left out; the others go on.
    Pack { doc_id: String, message: String },
    /// The cloud refused, or could not be reached.
    Cloud(CloudSyncError),
}

impl DocSyncError {
    /// Whether the next document would fail the same way, so the run should stop instead of
    /// trying every one: the cloud cannot be used right now, or the answer is not understood.
    /// The errors of one document (its files, its size, the total limit, its link) do not stop it.
    pub fn stops_the_run(&self) -> bool {
        match self {
            Self::Database(_) | Self::Pack { .. } => false,
            Self::Cloud(e) => !matches!(
                e,
                CloudSyncError::InvalidInput(_)
                    | CloudSyncError::ObjectTooLarge { .. }
                    | CloudSyncError::QuotaExceeded { .. }
                    | CloudSyncError::UploadLinkExpired
                    | CloudSyncError::ObjectChanged
            ),
        }
    }
}

impl fmt::Display for DocSyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(e) => write!(f, "could not read the document database: {e}"),
            Self::Pack { doc_id, message } => write!(f, "could not pack document {doc_id}: {message}"),
            Self::Cloud(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DocSyncError {}

impl From<CloudSyncError> for DocSyncError {
    fn from(e: CloudSyncError) -> Self {
        Self::Cloud(e)
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_error.rs"]
mod tests;
