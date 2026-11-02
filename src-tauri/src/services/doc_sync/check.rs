//! What a sync run would do now, asked without doing it: the answer for "is there anything to
//! sync". It lists the cloud, reads this machine's versions and counts; nothing is sent, nothing
//! is written, and the gate is not taken, so it can be asked while a run is going on.

use serde_json::{json, Value};

use super::compare;
use super::error::DocSyncError;
use super::plan::SyncAction;
use super::summary::not_made;
use crate::services::cloud_sync::{self, CloudDocs};
use crate::services::spark_data;

/// The check for the signed-in user, against the real cloud. Blocking: run it on a worker thread.
/// Answers like [`check_with`].
pub fn check_now() -> Value {
    check_with(cloud_sync::shared())
}

/// The check against `cloud`, on this machine's database.
///
/// The answer is `{ ok, uploads, pulls }` when the comparison was made: the number of documents a
/// run would send and the number it would bring back. When it could not be made it is
/// `{ error, _status }`, the same as the answer of a run that could not be made (see
/// [`not_made`]): nobody is signed in (401), the session is not accepted (401), the cloud cannot
/// be used (502), or this machine's database cannot be read (500).
pub fn check_with(cloud: &dyn CloudDocs) -> Value {
    let outcome = spark_data::open().map_err(DocSyncError::Database).and_then(|conn| compare(&conn, cloud));
    match outcome {
        Ok(actions) => counts(&actions),
        Err(error) => not_made(&error),
    }
}

fn counts(actions: &[SyncAction]) -> Value {
    let uploads = actions.iter().filter(|action| matches!(action, SyncAction::Upload { .. })).count();
    let pulls = actions.iter().filter(|action| matches!(action, SyncAction::Pull { .. })).count();
    json!({ "ok": true, "uploads": uploads, "pulls": pulls })
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_check.rs"]
mod tests;
