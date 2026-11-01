//! The sync the app runs when the user asks for one: the gate, this machine's database and
//! folders, the signed-in cloud, the search index, the log.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::cancel::Cancellable;
use super::gate::SyncGate;
use super::journal::{self, Line};
use super::logged::Logged;
use super::pull::PulledDoc;
use super::run::run_sync;
use super::summary::{busy, not_made, summary};
use crate::config::paths;
use crate::services::cloud_sync::{self, CloudDocs};
use crate::services::search_index;
use crate::services::spark_data;

/// The workspaces the pulled documents are in, each with the ids to re-index, in order of
/// workspace id.
pub(super) fn index_batches(pulled: &[PulledDoc]) -> Vec<(String, Vec<String>)> {
    let mut batches: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for doc in pulled {
        batches.entry(&doc.workspace_id).or_default().push(doc.doc_id.clone());
    }
    batches.into_iter().map(|(workspace, ids)| (workspace.to_string(), ids)).collect()
}

/// A pull changed the content of these documents, and the index is only kept in step by the
/// app's own writes, so it is told here. A failure of the index does not fail the sync.
fn refresh_index(pulled: &[PulledDoc]) {
    for (workspace_id, ids) in index_batches(pulled) {
        let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
        search_index::sync_workspace_docs_best_effort(&workspace_id, &ids);
    }
}

/// One sync for the signed-in user, against the real cloud. Blocking: run it on a worker thread.
/// Answers like [`sync_with`].
pub fn sync_now(gate: &SyncGate) -> Value {
    sync_with(gate, cloud_sync::shared())
}

/// Asks the sync that is going on to stop, and answers `{ ok: true, cancelling }`: `cancelling` is
/// false when no sync is going on, and then nothing is remembered. The sync stops before its next
/// transfer; the document it is transferring is finished (see [`sync_with`] for the answer it
/// gives).
pub fn cancel_now(gate: &SyncGate) -> Value {
    json!({ "ok": true, "cancelling": gate.cancel() })
}

/// One sync against `cloud`, on this machine's database and workspace folders.
///
/// The answer is `{ ok, uploaded, pulled, skipped, ... }` when a run was made (see
/// [`summary`]), even when part of it went wrong; or `{ error, _status }` when none was: nobody
/// is signed in (401), the session is not accepted (401), the cloud cannot be used (502), a run
/// is already going on (409), or this machine's database cannot be read (500). A sync the user
/// cancelled (see [`cancel_now`]) is a run that was made, with `cancelled: true`.
pub fn sync_with(gate: &SyncGate, cloud: &dyn CloudDocs) -> Value {
    let (answer, lines) = sync_and_journal(gate, cloud, &|line| journal::write(&[line]));
    journal::write(&lines);
    answer
}

/// The sync of [`sync_with`], with the log lines it earns when it ends handed back instead of
/// written, so that the lines can be read. The lines of what happens on the way (the run began, a
/// document went across) go to `live` as they happen: a run can be long, and can be cut short.
pub(super) fn sync_and_journal(
    gate: &SyncGate,
    cloud: &dyn CloudDocs,
    live: &dyn Fn(Line),
) -> (Value, Vec<Line>) {
    let Some(run) = gate.try_begin() else {
        return (busy(), Vec::new());
    };
    live(journal::line_started());
    let logged = Logged::new(cloud, live);
    let cloud = Cancellable::new(&logged, &run);
    let outcome = spark_data::open()
        .map_err(super::DocSyncError::Database)
        .and_then(|conn| {
            let root = paths::workspaces_root()
                .map_err(|e| super::DocSyncError::Database(format!("workspaces folder: {e:?}")))?;
            run_sync(&conn, &root, &cloud)
        });
    match outcome {
        Ok(report) => {
            refresh_index(&report.pull.pulled);
            (summary(&report), journal::lines_of(&report))
        }
        Err(error) => (not_made(&error), vec![journal::line_not_made(&error)]),
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_now.rs"]
mod tests;
