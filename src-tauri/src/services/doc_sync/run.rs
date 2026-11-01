//! One sync run: compare once, send what is newer here, then bring what is newer in the cloud.

use std::path::Path;

use rusqlite::Connection;

use super::error::DocSyncError;
use super::plan::SyncAction;
use super::pull::{pull_downloads, PullReport};
use super::push::{push_uploads, PushReport};
use super::compare;
use crate::services::cloud_sync::CloudDocs;

/// What one run did. Every document of the plan is in exactly one place: skipped, one of the
/// lists of the push report, or one of the lists of the pull report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Documents both sides hold at the same version.
    pub skipped: Vec<String>,
    pub push: PushReport,
    pub pull: PullReport,
}

/// Compares this machine with the cloud, runs the uploads, then the pulls.
///
/// Uploads go first: they put this machine's work in the cloud, which a pull never touches. When
/// the uploads were stopped by an error that would hit every document (the cloud cannot be
/// reached, the session is refused), the pulls are not tried, and they are reported as such.
///
/// There is no retry here. A stopped run is reported as it is; running again plans from the
/// versions as they are by then, so everything that went through is skipped and the run goes on
/// with the rest.
///
/// `Err` only when the comparison could not be made, and then nothing was sent or written.
pub fn run_sync(
    conn: &Connection,
    workspaces_root: &Path,
    cloud: &dyn CloudDocs,
) -> Result<SyncReport, DocSyncError> {
    let actions = compare(conn, cloud)?;
    let skipped = actions
        .iter()
        .filter(|a| matches!(a, SyncAction::Skip { .. }))
        .map(|a| a.doc_id().to_string())
        .collect();
    let push = push_uploads(conn, workspaces_root, cloud, &actions);
    let pull = if push.stopped.is_some() {
        PullReport {
            not_tried: actions
                .iter()
                .filter(|a| matches!(a, SyncAction::Pull { .. }))
                .map(|a| a.doc_id().to_string())
                .collect(),
            ..PullReport::default()
        }
    } else {
        pull_downloads(conn, workspaces_root, cloud, &actions)
    };
    Ok(SyncReport { skipped, push, pull })
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_run.rs"]
mod tests;
