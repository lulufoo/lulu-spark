//! Document sync (L4): compare this machine's documents with the cloud's list, then send what is
//! newer here. The client is the source of truth: only version numbers are compared.
//!
//! - [`plan`]: the comparison, a pure function from two lists to one action per document.
//! - [`compare`]: lists both sides and plans.
//! - [`upload_doc`], [`push_uploads`]: run the upload actions.
//! - [`pull_doc`], [`pull_downloads`]: run the pull actions.
//!
//! The database connection and the folder of workspace folders come from the caller, and so does
//! the cloud ([`CloudDocs`]).

mod error;
mod landing;
mod plan;
mod pull;
mod push;

use rusqlite::Connection;

pub use error::DocSyncError;
pub use plan::{plan, LocalDoc, SyncAction};
pub use pull::{pull_doc, pull_downloads, PullOutcome, PullReport, PulledDoc};
pub use push::{push_uploads, upload_doc, PushReport, UploadOutcome, UploadedDoc};

use crate::services::cloud_sync::CloudDocs;
use crate::services::spark_data::list_doc_versions;

/// What a sync run would do now: the cloud's list against the local `document` table, every
/// workspace included. The cloud is asked first, so the local side is as fresh as can be when the
/// decision is made.
pub fn compare(conn: &Connection, cloud: &dyn CloudDocs) -> Result<Vec<SyncAction>, DocSyncError> {
    let remote = cloud.list_documents()?;
    let local: Vec<LocalDoc> = list_doc_versions(conn)
        .map_err(DocSyncError::Database)?
        .into_iter()
        .map(|(doc_id, version)| LocalDoc { doc_id, version })
        .collect();
    Ok(plan(&local, &remote))
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync.rs"]
mod tests;
