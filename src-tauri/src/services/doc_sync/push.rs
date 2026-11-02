//! Running the upload actions of a plan: pack the document, send it, with its version.

use std::path::Path;

use rusqlite::Connection;

use super::error::DocSyncError;
use super::plan::SyncAction;
use crate::services::cloud_sync::CloudDocs;
use crate::services::doc_zip::pack_doc;
use crate::services::docs::workspace_dir;
use crate::services::spark_data::{find_doc, FoundDoc};

/// What became of one upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadOutcome {
    /// The package of `bytes` bytes is in the cloud, stamped `version`.
    Uploaded { version: i64, bytes: usize },
    /// The document is no longer here (deleted after the plan was made): nothing to send.
    Gone,
}

/// Sends one document. The row is read afresh, so the version is the document's now, not the one
/// the plan saw; it is read before the files, so the bytes are at least as new as the version
/// they carry. If the files change meanwhile, the version is one behind them and the next run
/// sends the document again, which loses nothing.
///
/// `workspaces_root` is the folder that holds the workspace folders.
pub fn upload_doc(
    conn: &Connection,
    workspaces_root: &Path,
    cloud: &dyn CloudDocs,
    doc_id: &str,
) -> Result<UploadOutcome, DocSyncError> {
    let Some(FoundDoc { workspace, doc }) = find_doc(conn, doc_id).map_err(DocSyncError::Database)? else {
        return Ok(UploadOutcome::Gone);
    };
    let pack_error = |message: String| DocSyncError::Pack { doc_id: doc_id.to_string(), message };
    let root = workspace_dir(workspaces_root, &workspace.id)
        .map_err(|e| pack_error(e["error"].as_str().unwrap_or("invalid workspace").to_string()))?;
    let package = pack_doc(&root, &doc).map_err(pack_error)?;
    cloud.upload(doc_id, &package, doc.version)?;
    Ok(UploadOutcome::Uploaded { version: doc.version, bytes: package.len() })
}

/// One document that went up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadedDoc {
    pub doc_id: String,
    pub version: i64,
    pub bytes: usize,
}

/// The result of [`push_uploads`]: every upload action is in exactly one of these places.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushReport {
    pub uploaded: Vec<UploadedDoc>,
    pub gone: Vec<String>,
    /// Documents that could not be sent for a reason of their own. The run went on without them.
    pub failed: Vec<(String, DocSyncError)>,
    /// The error that stopped the run (see [`DocSyncError::stops_the_run`]), with the document it
    /// happened on.
    pub stopped: Option<(String, DocSyncError)>,
    /// Upload actions after the one that stopped the run.
    pub not_tried: Vec<String>,
}

/// Runs the `Upload` actions of `actions` in order; other actions are not its business. A
/// failure of one document is recorded and the run goes on; an error that would hit every
/// document stops it, so a lost connection costs one timeout, not one per document.
pub fn push_uploads(
    conn: &Connection,
    workspaces_root: &Path,
    cloud: &dyn CloudDocs,
    actions: &[SyncAction],
) -> PushReport {
    let mut report = PushReport::default();
    for action in actions {
        let SyncAction::Upload { doc_id, .. } = action else { continue };
        if report.stopped.is_some() {
            report.not_tried.push(doc_id.clone());
            continue;
        }
        match upload_doc(conn, workspaces_root, cloud, doc_id) {
            Ok(UploadOutcome::Uploaded { version, bytes }) => {
                report.uploaded.push(UploadedDoc { doc_id: doc_id.clone(), version, bytes })
            }
            Ok(UploadOutcome::Gone) => report.gone.push(doc_id.clone()),
            Err(e) if e.stops_the_run() => report.stopped = Some((doc_id.clone(), e)),
            Err(e) => report.failed.push((doc_id.clone(), e)),
        }
    }
    report
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_push.rs"]
mod tests;
