//! Cloud Sync service (L4): the client side of the cloud-sync Worker and Google Cloud Storage (L5).
//!
//! - `credentials`: exchanges the signed-in session for a short-lived, read-only token for the
//!   user's bucket, and keeps it until shortly before it expires.
//! - `list_documents`: the `(document id, version)` pairs the cloud holds.
//! - `upload`, `read`, `delete`: the bytes of one document, by document id. Upload goes through a
//!   signed link from the Worker, read uses the storage token, delete goes through the Worker.
//!
//! - `CloudDocs`: the part of the above a sync run uses, as a trait, so the run can be tested
//!   against a stand-in cloud.
//! - `delete_cloud_copy`: `delete` for a document the user deletes; a user who is not signed in
//!   has nothing to remove there.
//!
//! The session is only read from the vault. This service never writes it and never refreshes it:
//! the UI refreshes the session before a sync starts. No long-lived key is held here.

mod api;
mod client;
mod error;
mod objects;
mod types;

use std::sync::OnceLock;

pub use api::{delete_cloud_copy, CloudDocs};
pub use client::{Clock, CloudSync, SessionSource};
pub use error::CloudSyncError;
pub use types::{CloudDoc, Credentials};

/// The deployed cloud-sync Worker (`workers/cloud-sync`). A public address, not a secret.
pub const WORKER_URL: &str = "https://cloud-sync.luluspark.app";

/// Google Cloud Storage. Reads go here, and an upload link must point here.
pub const STORAGE_URL: &str = "https://storage.googleapis.com";

/// The one instance of the app: it owns the token cache.
pub fn shared() -> &'static CloudSync {
    static SERVICE: OnceLock<CloudSync> = OnceLock::new();
    SERVICE.get_or_init(CloudSync::production)
}

#[cfg(test)]
#[path = "../../unit-tests/services/cloud_sync.rs"]
mod tests;
