//! What the UI is told about a run: the counts, and which documents went wrong and why.

use serde_json::{json, Value};

use super::error::DocSyncError;
use super::run::SyncReport;
use crate::services::cloud_sync::CloudSyncError;

fn failure(doc_id: &str, op: &str, error: &DocSyncError) -> Value {
    json!({ "id": doc_id, "op": op, "error": error.to_string() })
}

/// The answer of a run that was made: `{ ok, uploaded, pulled, skipped, kept, gone, not_tried,
/// failed, stopped }`.
///
/// - `uploaded`, `pulled`: documents that went up, documents that came down.
/// - `skipped`: documents both sides hold at the same version.
/// - `kept`: documents the pull found this machine had caught up with meanwhile.
/// - `gone`: documents deleted here while the run was going.
/// - `failed`: `{ id, op, error }` for each document that could not be sent or brought for a
///   reason of its own; `op` is `upload` or `pull`. The run went on without them.
/// - `stopped`: `null`, or `{ id, op, error }` for the error that ended the run early (the cloud
///   cannot be reached, the session is refused). `not_tried` documents were left for the next run.
pub fn summary(report: &SyncReport) -> Value {
    let SyncReport { skipped, push, pull } = report;
    let failed: Vec<Value> = push
        .failed
        .iter()
        .map(|(id, e)| failure(id, "upload", e))
        .chain(pull.failed.iter().map(|(id, e)| failure(id, "pull", e)))
        .collect();
    let stopped = match (&push.stopped, &pull.stopped) {
        (Some((id, e)), _) => failure(id, "upload", e),
        (None, Some((id, e))) => failure(id, "pull", e),
        (None, None) => Value::Null,
    };
    json!({
        "ok": true,
        "uploaded": push.uploaded.len(),
        "pulled": pull.pulled.len(),
        "skipped": skipped.len(),
        "kept": pull.kept.len(),
        "gone": push.gone.len(),
        "not_tried": push.not_tried.len() + pull.not_tried.len(),
        "failed": failed,
        "stopped": stopped,
    })
}

/// The HTTP-like status of a run that could not be made: 401 when the user has to sign in again,
/// 502 when the cloud could not be used, 500 when this machine's own data is the trouble.
fn status_of(error: &DocSyncError) -> u16 {
    match error {
        DocSyncError::Cloud(
            CloudSyncError::NotSignedIn | CloudSyncError::SessionExpired | CloudSyncError::Unauthorized,
        ) => 401,
        DocSyncError::Cloud(_) => 502,
        DocSyncError::Database(_) | DocSyncError::Pack { .. } | DocSyncError::Unpack { .. } => 500,
    }
}

/// The answer of a run that could not be made, `{ error, _status }` like the other commands:
/// nothing was sent or written.
pub fn not_made(error: &DocSyncError) -> Value {
    json!({ "error": format!("Cannot sync: {error}"), "_status": status_of(error) })
}

/// The answer when a run is already going on.
pub fn busy() -> Value {
    json!({ "error": "Cannot sync: a sync is already running", "_status": 409 })
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_summary.rs"]
mod tests;
