use serde_json::json;

use super::*;
use crate::services::cloud_sync::CloudSyncError;
use crate::services::doc_sync::{PulledDoc, UploadedDoc};

fn events(lines: &[Line]) -> Vec<&'static str> {
    lines.iter().map(|l| l.event).collect()
}

#[test]
fn a_clean_run_writes_one_info_line_with_the_counts() {
    // Every count is told apart from the others, so that a count put in the wrong place shows.
    let mut report = SyncReport::default();
    report.skipped = (0..5).map(|n| format!("s{n}")).collect();
    report.push.uploaded = (0..3).map(|n| UploadedDoc { doc_id: format!("u{n}"), version: 1, bytes: 3 }).collect();
    report.pull.pulled = (0..2)
        .map(|n| PulledDoc { doc_id: format!("p{n}"), version: 2, bytes: 3, created: true, workspace_id: "w".into() })
        .collect();
    report.pull.kept = (0..4).map(|n| format!("k{n}")).collect();
    report.push.gone = vec!["g1".into()];

    let lines = lines_of(&report);

    assert_eq!(
        lines,
        [Line {
            level: Level::Info,
            event: "sync.finished",
            params: json!({
                "uploaded": 3, "pulled": 2, "skipped": 5, "kept": 4, "gone": 1,
                "failed": 0, "not_tried": 0, "stopped": false, "cancelled": false,
            }),
        }]
    );
}

#[test]
fn each_failed_document_is_logged_with_its_reason_and_the_run_ends_with_a_warning() {
    let mut report = SyncReport::default();
    report.push.failed = vec![(
        "d1".into(),
        DocSyncError::Pack { doc_id: "d1".into(), message: "raw/x.md: not found".into() },
    )];
    report.pull.failed = vec![("d2".into(), DocSyncError::Cloud(CloudSyncError::ObjectNotFound))];

    let lines = lines_of(&report);

    assert_eq!(events(&lines), ["sync.doc_failed", "sync.doc_failed", "sync.finished"]);
    assert_eq!(
        lines[0],
        Line {
            level: Level::Warn,
            event: "sync.doc_failed",
            params: json!({ "doc_id": "d1", "op": "upload", "error": "could not pack document d1: raw/x.md: not found" }),
        }
    );
    assert_eq!(
        lines[1].params,
        json!({ "doc_id": "d2", "op": "pull", "error": "the document is not in the cloud" })
    );
    assert_eq!(lines[2].level, Level::Warn);
    assert_eq!(lines[2].params["failed"], json!(2));
}

#[test]
fn a_run_that_stopped_logs_where_and_why() {
    let mut report = SyncReport::default();
    report.push.stopped = Some(("u1".into(), DocSyncError::Cloud(CloudSyncError::Network("down".into()))));
    report.push.not_tried = vec!["u2".into()];
    report.pull.not_tried = vec!["p1".into(), "p2".into()];

    let lines = lines_of(&report);

    assert_eq!(events(&lines), ["sync.stopped", "sync.finished"]);
    assert_eq!(
        lines[0].params,
        json!({ "doc_id": "u1", "op": "upload", "error": "cloud sync not reachable: down" })
    );
    assert_eq!(lines[1].level, Level::Warn);
    assert_eq!(lines[1].params["stopped"], json!(true));
    assert_eq!(lines[1].params["not_tried"], json!(3));
}

#[test]
fn a_run_that_stopped_in_the_pulls_logs_the_pull() {
    let mut report = SyncReport::default();
    report.pull.stopped = Some(("p1".into(), DocSyncError::Cloud(CloudSyncError::SessionExpired)));

    let lines = lines_of(&report);

    assert_eq!(lines[0].params, json!({ "doc_id": "p1", "op": "pull", "error": "sign-in session expired" }));
    assert_eq!(lines[1].params["stopped"], json!(true));
}

fn cancelled(doc: &str) -> Option<(String, DocSyncError)> {
    Some((doc.to_string(), DocSyncError::Cloud(CloudSyncError::Cancelled)))
}

#[test]
fn a_run_the_user_cancelled_is_logged_as_that_and_is_no_trouble() {
    let mut report = SyncReport::default();
    report.push.uploaded = vec![UploadedDoc { doc_id: "u1".into(), version: 1, bytes: 3 }];
    report.push.stopped = cancelled("u2");
    report.push.not_tried = vec!["u3".into()];
    report.pull.not_tried = vec!["p1".into(), "p2".into()];

    let lines = lines_of(&report);

    assert_eq!(events(&lines), ["sync.cancelled", "sync.finished"]);
    assert_eq!(
        lines[0],
        Line {
            level: Level::Info,
            event: "sync.cancelled",
            params: json!({ "doc_id": "u2", "op": "upload", "error": "the sync was cancelled" }),
        }
    );
    assert_eq!(lines[1].level, Level::Info, "a cancel is the user's choice, not a warning");
    assert_eq!(lines[1].params["cancelled"], json!(true));
    assert_eq!(lines[1].params["stopped"], json!(false), "a cancel is not an error stop");
    assert_eq!(lines[1].params["not_tried"], json!(4), "the document it stopped on was never started");
    assert_eq!(lines[1].params["uploaded"], json!(1));
}

#[test]
fn a_cancelled_run_still_warns_about_the_documents_that_failed_before_the_cancel() {
    let mut report = SyncReport::default();
    report.push.failed = vec![("d1".into(), DocSyncError::Cloud(CloudSyncError::ObjectNotFound))];
    report.pull.stopped = cancelled("p1");

    let lines = lines_of(&report);

    assert_eq!(events(&lines), ["sync.doc_failed", "sync.cancelled", "sync.finished"]);
    assert_eq!(lines[1].params["op"], json!("pull"));
    assert_eq!(lines[2].level, Level::Warn);
}

#[test]
fn a_run_that_could_not_be_made_writes_a_warning() {
    let line = line_not_made(&DocSyncError::Cloud(CloudSyncError::NotSignedIn));

    assert_eq!(
        line,
        Line { level: Level::Warn, event: "sync.not_made", params: json!({ "error": "not signed in" }) }
    );
}

#[test]
fn every_event_name_is_one_the_app_log_accepts() {
    let mut report = SyncReport::default();
    report.push.failed = vec![("d".into(), DocSyncError::Database("x".into()))];
    report.push.stopped = Some(("d".into(), DocSyncError::Database("x".into())));
    let mut lines = lines_of(&report);
    report.push.stopped = cancelled("d");
    lines.extend(lines_of(&report));
    lines.push(line_not_made(&DocSyncError::Database("x".into())));

    assert!(app_log::token_ok(BUSINESS));
    for line in lines {
        assert!(app_log::token_ok(line.event), "{}", line.event);
    }
}

#[test]
fn the_lines_of_what_goes_on_are_info_lines_with_their_fields() {
    let cases = [
        (line_started(), "sync.started", json!({})),
        (line_listed(4, 120), "sync.listed", json!({ "cloud_docs": 4, "ms": 120 })),
        (
            line_uploaded("d1", 3, 2048, 340),
            "sync.doc_uploaded",
            json!({ "doc_id": "d1", "version": 3, "bytes": 2048, "ms": 340 }),
        ),
        (
            line_downloaded("d2", 512, 90),
            "sync.doc_downloaded",
            json!({ "doc_id": "d2", "bytes": 512, "ms": 90 }),
        ),
    ];
    for (line, event, params) in cases {
        assert_eq!(line, Line { level: Level::Info, event, params }, "{event}");
    }
}
