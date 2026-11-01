//! Document sync (L4): compare this machine's documents with the cloud's list, then send what is
//! newer here. The client is the source of truth: only version numbers are compared.
//!
//! - [`plan`]: the comparison, a pure function from two lists to one action per document.
//! - [`compare`]: lists both sides and plans.
//! - [`upload_doc`], [`push_uploads`]: run the upload actions.
//! - [`pull_doc`], [`pull_downloads`]: run the pull actions.
//! - [`run_sync`]: one run, the three above in turn; [`SyncReport`] tells what it did.
//! - [`sync_now`], [`sync_with`]: the run the app makes when the user asks for one, with the
//!   gate ([`SyncGate`]), the search index, the log, and the answer for the UI ([`summary`]).
//!
//! Below `sync_now` the database connection and the folder of workspace folders come from the
//! caller, and so does the cloud ([`CloudDocs`]).

mod error;
mod gate;
mod journal;
mod landing;
mod now;
mod plan;
mod pull;
mod push;
mod run;
mod summary;

use rusqlite::Connection;

pub use error::DocSyncError;
pub use gate::{SyncGate, SyncRun};
pub use now::{sync_now, sync_with};
pub use plan::{plan, LocalDoc, SyncAction};
pub use pull::{pull_doc, pull_downloads, PullOutcome, PullReport, PulledDoc};
pub use push::{push_uploads, upload_doc, PushReport, UploadOutcome, UploadedDoc};
pub use run::{run_sync, SyncReport};
pub use summary::summary;

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
