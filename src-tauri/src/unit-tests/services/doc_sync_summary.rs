use serde_json::json;

use super::*;
use crate::services::doc_sync::{PulledDoc, UploadedDoc};

fn upload_failure(doc: &str) -> (String, DocSyncError) {
    (doc.to_string(), DocSyncError::Pack { doc_id: doc.to_string(), message: "raw/x.md: not found".into() })
}

fn pull_failure(doc: &str) -> (String, DocSyncError) {
    (doc.to_string(), DocSyncError::Cloud(CloudSyncError::ObjectNotFound))
}

fn full_report() -> SyncReport {
    let mut report = SyncReport::default();
    report.skipped = vec!["s1".into(), "s2".into(), "s3".into(), "s4".into()];
    report.push.uploaded = vec![
        UploadedDoc { doc_id: "u1".into(), version: 2, bytes: 10 },
        UploadedDoc { doc_id: "u2".into(), version: 3, bytes: 20 },
    ];
    report.push.gone = vec!["g1".into()];
    report.push.failed = vec![upload_failure("uf1")];
    report.push.not_tried = vec!["un1".into(), "un2".into()];
    report.pull.pulled = vec![
        PulledDoc { doc_id: "p1".into(), version: 4, bytes: 5, created: true, workspace_id: "w".into() },
        PulledDoc { doc_id: "p2".into(), version: 6, bytes: 5, created: false, workspace_id: "w".into() },
        PulledDoc { doc_id: "p3".into(), version: 8, bytes: 5, created: true, workspace_id: "w".into() },
    ];
    report.pull.kept = vec!["k1".into(), "k2".into()];
    report.pull.failed = vec![pull_failure("pf1"), pull_failure("pf2")];
    report.pull.not_tried = vec!["pn1".into()];
    report
}

#[test]
fn every_count_is_its_own_list() {
    let answer = summary(&full_report());

    assert_eq!(answer["ok"], json!(true));
    assert_eq!(answer["uploaded"], json!(2));
    assert_eq!(answer["pulled"], json!(3));
    assert_eq!(answer["skipped"], json!(4));
    assert_eq!(answer["kept"], json!(2));
    assert_eq!(answer["gone"], json!(1));
    assert_eq!(answer["not_tried"], json!(3), "the not tried uploads and pulls together");
}

#[test]
fn each_failed_document_comes_with_its_operation_and_its_reason() {
    let answer = summary(&full_report());

    assert_eq!(
        answer["failed"],
        json!([
            { "id": "uf1", "op": "upload", "error": "could not pack document uf1: raw/x.md: not found" },
            { "id": "pf1", "op": "pull", "error": "the document is not in the cloud" },
            { "id": "pf2", "op": "pull", "error": "the document is not in the cloud" },
        ])
    );
}

#[test]
fn a_run_that_was_not_stopped_says_so() {
    assert_eq!(summary(&full_report())["stopped"], json!(null));
    assert_eq!(summary(&SyncReport::default())["stopped"], json!(null));
}

#[test]
fn a_run_stopped_in_the_uploads_names_the_document_and_the_reason() {
    let mut report = full_report();
    report.push.stopped = Some(("u9".into(), DocSyncError::Cloud(CloudSyncError::Network("down".into()))));

    assert_eq!(
        summary(&report)["stopped"],
        json!({ "id": "u9", "op": "upload", "error": "cloud sync not reachable: down" })
    );
}

#[test]
fn a_run_stopped_in_the_pulls_names_the_document_and_the_reason() {
    let mut report = full_report();
    report.pull.stopped = Some(("p9".into(), DocSyncError::Cloud(CloudSyncError::SessionExpired)));

    assert_eq!(
        summary(&report)["stopped"],
        json!({ "id": "p9", "op": "pull", "error": "sign-in session expired" })
    );
}

#[test]
fn an_empty_run_is_all_zero_and_ok() {
    assert_eq!(
        summary(&SyncReport::default()),
        json!({
            "ok": true, "uploaded": 0, "pulled": 0, "skipped": 0, "kept": 0, "gone": 0,
            "not_tried": 0, "failed": [], "stopped": null,
        })
    );
}

#[test]
fn a_run_that_could_not_be_made_says_why_and_how_to_fix_it() {
    let cases = [
        (DocSyncError::Cloud(CloudSyncError::NotSignedIn), 401, "Cannot sync: not signed in"),
        (DocSyncError::Cloud(CloudSyncError::SessionExpired), 401, "Cannot sync: sign-in session expired"),
        (
            DocSyncError::Cloud(CloudSyncError::Unauthorized),
            401,
            "Cannot sync: cloud sync refused the sign-in session (401)",
        ),
        (
            DocSyncError::Cloud(CloudSyncError::Network("connection refused".into())),
            502,
            "Cannot sync: cloud sync not reachable: connection refused",
        ),
        (
            DocSyncError::Cloud(CloudSyncError::Unavailable { status: 503, code: "unknown".into() }),
            502,
            "Cannot sync: cloud sync unavailable (503 unknown)",
        ),
        (
            DocSyncError::Cloud(CloudSyncError::Vault("locked".into())),
            502,
            "Cannot sync: could not read the sign-in session: locked",
        ),
        (
            DocSyncError::Database("no such table".into()),
            500,
            "Cannot sync: could not read the document database: no such table",
        ),
    ];
    for (error, status, message) in cases {
        let answer = not_made(&error);
        assert_eq!(answer, json!({ "error": message, "_status": status }), "{error:?}");
        assert_eq!(answer.get("ok"), None, "an error answer is not an ok answer");
    }
}

#[test]
fn a_second_run_is_refused_with_a_conflict() {
    assert_eq!(busy(), json!({ "error": "Cannot sync: a sync is already running", "_status": 409 }));
}
