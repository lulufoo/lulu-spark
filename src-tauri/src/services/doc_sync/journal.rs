//! What a run writes to the app log: how it ended, and each document that went wrong. The UI
//! gets the summary of the run; the log keeps the reasons.

use serde_json::{json, Value};

use super::error::DocSyncError;
use super::run::SyncReport;
use crate::services::app_log::{self, Level, Side};

/// The log business of every line below.
pub const BUSINESS: &str = "cloud_sync";

/// One line for the app log.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub level: Level,
    pub event: &'static str,
    pub params: Value,
}

fn doc_line(event: &'static str, doc_id: &str, op: &str, error: &DocSyncError) -> Line {
    Line {
        level: Level::Warn,
        event,
        params: json!({ "doc_id": doc_id, "op": op, "error": error.to_string() }),
    }
}

/// The lines of a run that was made: each failed document (`sync.doc_failed`), the error that
/// ended the run early (`sync.stopped`), and how it ended (`sync.finished`) with the counts.
pub fn lines_of(report: &SyncReport) -> Vec<Line> {
    let SyncReport { skipped, push, pull } = report;
    let mut lines: Vec<Line> = push
        .failed
        .iter()
        .map(|(id, e)| doc_line("sync.doc_failed", id, "upload", e))
        .chain(pull.failed.iter().map(|(id, e)| doc_line("sync.doc_failed", id, "pull", e)))
        .collect();
    if let Some((id, e)) = &push.stopped {
        lines.push(doc_line("sync.stopped", id, "upload", e));
    }
    if let Some((id, e)) = &pull.stopped {
        lines.push(doc_line("sync.stopped", id, "pull", e));
    }
    let troubled = !lines.is_empty();
    lines.push(Line {
        level: if troubled { Level::Warn } else { Level::Info },
        event: "sync.finished",
        params: json!({
            "uploaded": push.uploaded.len(),
            "pulled": pull.pulled.len(),
            "skipped": skipped.len(),
            "kept": pull.kept.len(),
            "gone": push.gone.len(),
            "failed": push.failed.len() + pull.failed.len(),
            "not_tried": push.not_tried.len() + pull.not_tried.len(),
            "stopped": push.stopped.is_some() || pull.stopped.is_some(),
        }),
    });
    lines
}

/// The line of a run that could not be made.
pub fn line_not_made(error: &DocSyncError) -> Line {
    Line { level: Level::Warn, event: "sync.not_made", params: json!({ "error": error.to_string() }) }
}

pub fn write(lines: &[Line]) {
    for line in lines {
        app_log::log(BUSINESS, line.event, None, Some(line.params.clone()), Side::Host, line.level);
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_journal.rs"]
mod tests;
