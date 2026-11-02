//! The record of a run, for the screen to read while the run goes on and after it: one entry for
//! each document the run plans to send or bring back, and how far each one got.
//!
//! The record lives in memory and is shared (clones share it), like the gate. The loops write to it
//! as they go; the UI asks for all of it at once, again and again, and replaces what it had. There
//! is no push from here, and no "what about this one document" question to get wrong.
//!
//! - A run clears the record when it begins (see `SyncGate::try_begin`). It keeps what it wrote
//!   when it ends, until the next run begins, so the screen can read the end of a run once more.
//! - The comparison fills it: every upload, then every download, each in order of document id, all
//!   `NotStarted`. Documents both sides hold at the same version are not in it.
//! - A document is `Running` while it is transferred, then `Done` or `Failed` with the reason. A
//!   run that stops (the user cancels, an error would hit every document) leaves the documents it
//!   did not reach `NotStarted`; the document an error stopped it on is `Failed`, and the one a
//!   cancel stopped it on is `NotStarted`, because that transfer never began.
//! - A document that is no longer here when its upload comes, or that this machine has caught up
//!   with while a download was coming, is taken out: nothing went across, and the summary of the
//!   run counts it as `gone` or `kept`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use rusqlite::Connection;
use serde_json::{json, Value};

use super::error::DocSyncError;
use super::plan::SyncAction;
use crate::services::spark_data::{get_doc, DocRow};

/// How many characters of a document id stand for it when nothing better names it.
const SHORT_ID: usize = 8;

/// Which way a document goes. Spelled as in the summary of a run (`op`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Upload,
    Pull,
}

impl Op {
    fn as_str(self) -> &'static str {
        match self {
            Self::Upload => "upload",
            Self::Pull => "pull",
        }
    }
}

/// How far one document got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    NotStarted,
    Running,
    Done,
    Failed,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }
}

/// One document of the run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub doc_id: String,
    /// What the screen calls it: the title, else the file name, else the short id (see
    /// [`name_of`]). A document that is not here yet has only the short id, until it comes down.
    pub name: String,
    pub op: Op,
    pub status: Status,
    /// Why it failed, in the words of the log and of the summary. `None` unless `Failed`.
    pub reason: Option<String>,
    /// Whether this machine has the document: always for an upload; for a download, only when it
    /// replaces a document that is here, or once it has come down.
    pub local: bool,
}

/// All of the record at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Whether a run is going on, so that a reader can tell a record that is being written from
    /// one that was left by the last run.
    pub running: bool,
    pub entries: Vec<Entry>,
}

impl Snapshot {
    /// `{ ok, running, entries: [{ id, name, op, state, reason, local }] }`: `op` is `upload` or
    /// `pull`, `state` is `not_started`, `running`, `done` or `failed`, `reason` is a string or
    /// `null`.
    pub fn to_json(&self) -> Value {
        let entries: Vec<Value> = self
            .entries
            .iter()
            .map(|entry| {
                json!({
                    "id": entry.doc_id,
                    "name": entry.name,
                    "op": entry.op.as_str(),
                    "state": entry.status.as_str(),
                    "reason": entry.reason,
                    "local": entry.local,
                })
            })
            .collect();
        json!({ "ok": true, "running": self.running, "entries": entries })
    }
}

#[derive(Debug, Default)]
struct Roster {
    running: bool,
    entries: Vec<Entry>,
    /// Where each document is in `entries`, so that a run of thousands does not search for each.
    index: HashMap<String, usize>,
}

impl Roster {
    fn set(&mut self, entries: Vec<Entry>) {
        self.entries = entries;
        self.reindex();
    }

    fn reindex(&mut self) {
        self.index = self.entries.iter().enumerate().map(|(at, entry)| (entry.doc_id.clone(), at)).collect();
    }

    fn entry(&mut self, doc_id: &str) -> Option<&mut Entry> {
        let at = *self.index.get(doc_id)?;
        self.entries.get_mut(at)
    }
}

/// The shared record of a run. Clones share it.
#[derive(Debug, Clone, Default)]
pub struct SyncProgress {
    roster: Arc<Mutex<Roster>>,
}

impl SyncProgress {
    pub fn new() -> Self {
        Self::default()
    }

    /// A reader must not be locked out for good because a run panicked while it wrote.
    fn lock(&self) -> MutexGuard<'_, Roster> {
        self.roster.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// All of the record, as it is at this moment.
    pub fn snapshot(&self) -> Snapshot {
        let roster = self.lock();
        Snapshot { running: roster.running, entries: roster.entries.clone() }
    }

    /// A run begins: what the last one left is cleared, in the same step that says one is going on.
    pub(super) fn begin(&self) {
        let mut roster = self.lock();
        roster.running = true;
        roster.set(Vec::new());
    }

    /// The run is over. What it wrote stays until the next one begins.
    pub(super) fn end(&self) {
        self.lock().running = false;
    }

    /// The comparison is made: every upload, then every download, as the loops will take them.
    /// `actions` are in order of document id, which is the order of each of the two groups.
    pub(super) fn plan(&self, conn: &Connection, actions: &[SyncAction]) {
        let uploads = actions.iter().filter_map(|action| match action {
            SyncAction::Upload { doc_id, .. } => Some(entry_for(conn, doc_id, Op::Upload)),
            _ => None,
        });
        let pulls = actions.iter().filter_map(|action| match action {
            SyncAction::Pull { doc_id, .. } => Some(entry_for(conn, doc_id, Op::Pull)),
            _ => None,
        });
        let entries: Vec<Entry> = uploads.chain(pulls).collect();
        self.lock().set(entries);
    }

    /// The transfer of this document begins.
    pub(super) fn start(&self, doc_id: &str) {
        self.change(doc_id, |entry| entry.status = Status::Running);
    }

    /// This document went up.
    pub(super) fn done(&self, doc_id: &str) {
        self.change(doc_id, |entry| entry.status = Status::Done);
    }

    /// This document came down and is here: it is named by what the row says now.
    pub(super) fn landed(&self, conn: &Connection, doc_id: &str) {
        let name = get_doc(conn, doc_id).ok().flatten().map(|row| name_of(&row));
        self.change(doc_id, |entry| {
            entry.status = Status::Done;
            entry.local = true;
            if let Some(name) = name {
                entry.name = name;
            }
        });
    }

    /// This document could not be transferred, for a reason of its own. The run goes on.
    pub(super) fn fail(&self, doc_id: &str, error: &DocSyncError) {
        let reason = error.to_string();
        self.change(doc_id, |entry| {
            entry.status = Status::Failed;
            entry.reason = Some(reason);
        });
    }

    /// An error ended the run on this document. If the user cancelled, the transfer never began, so
    /// the document goes back to `NotStarted`; any other error is the document's failure.
    pub(super) fn stopped_on(&self, doc_id: &str, error: &DocSyncError) {
        if error.is_cancelled() {
            self.change(doc_id, |entry| entry.status = Status::NotStarted);
        } else {
            self.fail(doc_id, error);
        }
    }

    /// Nothing went across for this document, and it was not a failure (see the module notes).
    pub(super) fn forget(&self, doc_id: &str) {
        let mut roster = self.lock();
        let before = roster.entries.len();
        roster.entries.retain(|entry| entry.doc_id != doc_id);
        if roster.entries.len() != before {
            roster.reindex();
        }
    }

    fn change(&self, doc_id: &str, update: impl FnOnce(&mut Entry)) {
        if let Some(entry) = self.lock().entry(doc_id) {
            update(entry);
        }
    }
}

fn entry_for(conn: &Connection, doc_id: &str, op: Op) -> Entry {
    let row = get_doc(conn, doc_id).ok().flatten();
    Entry {
        doc_id: doc_id.to_string(),
        name: row.as_ref().map_or_else(|| short_id(doc_id), name_of),
        op,
        status: Status::NotStarted,
        reason: None,
        // Whatever is sent is here. A document that comes down is here only when it replaces one.
        local: op == Op::Upload || row.is_some(),
    }
}

/// What the screen calls a document: its title, else the name of its file, else the first
/// characters of its id. A document that came down from the cloud has no title (the package does
/// not carry one), so it is known by its file name.
pub(super) fn name_of(row: &DocRow) -> String {
    let title = row.title.as_deref().map(str::trim).filter(|title| !title.is_empty());
    let file = row.raw_path.rsplit('/').next().filter(|name| !name.is_empty());
    title.or(file).map_or_else(|| short_id(&row.doc_id), str::to_string)
}

fn short_id(doc_id: &str) -> String {
    doc_id.chars().take(SHORT_ID).collect()
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_progress.rs"]
mod tests;
