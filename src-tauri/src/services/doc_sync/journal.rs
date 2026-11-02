//! What a run writes to the app log: that it began, what the cloud list was, each document that
//! went across, each one that went wrong, and how it ended. The UI gets the summary of the run;
//! the log keeps the reasons and the times.

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

fn info(event: &'static str, params: Value) -> Line {
    Line { level: Level::Info, event, params }
}

/// A run began.
pub fn line_started() -> Line {
    info("sync.started", json!({}))
}

/// The cloud's list came back with `cloud_docs` documents, after `ms` milliseconds.
pub fn line_listed(cloud_docs: usize, ms: u64) -> Line {
    info("sync.listed", json!({ "cloud_docs": cloud_docs, "ms": ms }))
}

/// A document's package, `bytes` long, was written to the cloud as `version`, in `ms` milliseconds.
pub fn line_uploaded(doc_id: &str, version: i64, bytes: usize, ms: u64) -> Line {
    info("sync.doc_uploaded", json!({ "doc_id": doc_id, "version": version, "bytes": bytes, "ms": ms }))
}

/// A document's package, `bytes` long, was read from the cloud, in `ms` milliseconds.
pub fn line_downloaded(doc_id: &str, bytes: usize, ms: u64) -> Line {
    info("sync.doc_downloaded", json!({ "doc_id": doc_id, "bytes": bytes, "ms": ms }))
}

/// The lines of a run that was made: each failed document (`sync.doc_failed`), the error that
/// ended the run early (`sync.stopped`) or the user's cancel (`sync.cancelled`, not a trouble),
/// and how it ended (`sync.finished`) with the counts.
pub fn lines_of(report: &SyncReport) -> Vec<Line> {
    let SyncReport { skipped, push, pull } = report;
    let mut lines: Vec<Line> = push
        .failed
        .iter()
        .map(|(id, e)| doc_line("sync.doc_failed", id, "upload", e))
        .chain(pull.failed.iter().map(|(id, e)| doc_line("sync.doc_failed", id, "pull", e)))
        .collect();
    let cancelled = report.was_cancelled();
    let stop_line = |id: &str, op: &str, e: &DocSyncError| {
        if cancelled {
            Line { level: Level::Info, ..doc_line("sync.cancelled", id, op, e) }
        } else {
            doc_line("sync.stopped", id, op, e)
        }
    };
    if let Some((id, e)) = &push.stopped {
        lines.push(stop_line(id, "upload", e));
    }
    if let Some((id, e)) = &pull.stopped {
        lines.push(stop_line(id, "pull", e));
    }
    let troubled = lines.iter().any(|line| line.event != "sync.cancelled");
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
            "not_tried": report.not_tried(),
            "stopped": !cancelled && (push.stopped.is_some() || pull.stopped.is_some()),
            "cancelled": cancelled,
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
