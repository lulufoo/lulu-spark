use super::*;
use crate::services::cloud_sync::CloudSyncError;
use crate::services::doc_sync::tests::{id, Call, FakeCloud, World};

/// This machine and the cloud, set up so that one run has something of every kind to do:
///
/// | document | here | cloud | action |
/// |---|---|---|---|
/// | 1 | v5 | v2 | upload |
/// | 2 | v3 | v3 | skip |
/// | 3 | v1 | v4 | pull |
/// | 4 | v0 | none | upload |
/// | 9 | none | v7 | pull |
fn five_documents() -> (World, FakeCloud) {
    let elsewhere = World::new();
    elsewhere.add_full(3, 4);
    elsewhere.add_full(9, 7);
    let world = World::new();
    world.add(1, 5);
    world.add(2, 3);
    world.add(3, 1);
    world.add(4, 0);
    let cloud = FakeCloud::new()
        .with(&id(1), 2)
        .with(&id(2), 3)
        .with_package(&id(3), 4, elsewhere.package_of(3))
        .with_package(&id(9), 7, elsewhere.package_of(9));
    (world, cloud)
}

fn run(world: &World, cloud: &FakeCloud) -> Result<SyncReport, DocSyncError> {
    run_sync(&world.conn, &world.workspaces_root, cloud, &SyncProgress::new())
}

fn ids(report: &SyncReport) -> (Vec<String>, Vec<String>, Vec<String>) {
    (
        report.skipped.clone(),
        report.push.uploaded.iter().map(|d| d.doc_id.clone()).collect(),
        report.pull.pulled.iter().map(|d| d.doc_id.clone()).collect(),
    )
}

/// How many documents the report accounts for: each one of the plan is in exactly one place.
fn accounted(report: &SyncReport) -> usize {
    let SyncReport { skipped, push, pull } = report;
    skipped.len()
        + push.uploaded.len()
        + push.gone.len()
        + push.failed.len()
        + usize::from(push.stopped.is_some())
        + push.not_tried.len()
        + pull.pulled.len()
        + pull.kept.len()
        + pull.failed.len()
        + usize::from(pull.stopped.is_some())
        + pull.not_tried.len()
}

#[test]
fn a_run_sends_what_is_newer_here_and_brings_what_is_newer_there() {
    let (world, cloud) = five_documents();

    let report = run(&world, &cloud).expect("run");

    let (skipped, uploaded, pulled) = ids(&report);
    assert_eq!(skipped, [id(2)]);
    assert_eq!(uploaded, [id(1), id(4)]);
    assert_eq!(pulled, [id(3), id(9)]);
    assert_eq!(report.push.uploaded.iter().map(|d| d.version).collect::<Vec<_>>(), [5, 0]);
    assert_eq!(report.pull.pulled.iter().map(|d| d.version).collect::<Vec<_>>(), [4, 7]);
    assert!(report.pull.pulled.iter().all(|d| d.workspace_id == world.workspace_id));
    assert!(report.push.failed.is_empty() && report.pull.failed.is_empty());
    assert!(report.push.stopped.is_none() && report.pull.stopped.is_none());
    assert_eq!(accounted(&report), 5);

    assert_eq!(cloud.stored(&id(1)).expect("uploaded").0, 5);
    assert_eq!(cloud.stored(&id(4)).expect("uploaded").0, 0);
    assert_eq!(world.row_of(3).expect("pulled over").version, 4);
    assert_eq!(world.row_of(9).expect("pulled in").version, 7);
}

#[test]
fn the_uploads_go_before_the_pulls() {
    let (world, cloud) = five_documents();

    run(&world, &cloud).expect("run");

    let kinds: Vec<&str> = cloud
        .calls()
        .iter()
        .map(|c| match c {
            Call::List => "list",
            Call::Upload { .. } => "upload",
            Call::Read { .. } => "read",
            Call::Delete { .. } => "delete",
        })
        .collect();
    assert_eq!(kinds, ["list", "upload", "upload", "read", "read"]);
}

#[test]
fn running_again_finds_everything_skipped() {
    let (world, cloud) = five_documents();
    run(&world, &cloud).expect("first run");

    let again = run(&world, &cloud).expect("second run");

    let (skipped, uploaded, pulled) = ids(&again);
    assert_eq!(skipped, [id(1), id(2), id(3), id(4), id(9)]);
    assert!(uploaded.is_empty() && pulled.is_empty());
    assert_eq!(accounted(&again), 5);
}

#[test]
fn an_error_that_hits_every_upload_stops_the_run_and_the_pulls_are_not_tried() {
    let (world, cloud) = five_documents();
    let cloud = cloud.failing_upload(&id(1), CloudSyncError::Network("down".into()));

    let report = run(&world, &cloud).expect("run");

    assert_eq!(report.push.stopped.as_ref().map(|(doc, _)| doc.clone()), Some(id(1)));
    assert_eq!(report.push.not_tried, [id(4)]);
    assert_eq!(report.pull.not_tried, [id(3), id(9)], "the pulls are reported, not silently dropped");
    assert!(report.pull.pulled.is_empty() && report.pull.stopped.is_none());
    assert_eq!(accounted(&report), 5);
    assert!(!cloud.calls().iter().any(|c| matches!(c, Call::Read { .. })), "{:?}", cloud.calls());
    assert_eq!(world.row_of(3).expect("row").version, 1, "document 3 is as it was");
    assert!(world.row_of(9).is_none(), "document 9 is not here");
}

#[test]
fn a_document_that_cannot_be_sent_does_not_keep_the_others_from_coming() {
    let (world, cloud) = five_documents();
    let cloud = cloud.failing_upload(&id(1), CloudSyncError::QuotaExceeded { limit_bytes: 10, used_bytes: 10 });

    let report = run(&world, &cloud).expect("run");

    assert_eq!(report.push.failed.len(), 1);
    assert_eq!(report.push.failed[0].0, id(1));
    assert!(report.push.stopped.is_none());
    let (_, uploaded, pulled) = ids(&report);
    assert_eq!(uploaded, [id(4)]);
    assert_eq!(pulled, [id(3), id(9)]);
    assert_eq!(accounted(&report), 5);
}

#[test]
fn an_error_that_hits_every_pull_stops_the_pulls_and_keeps_what_was_sent() {
    let (world, cloud) = five_documents();
    let cloud = cloud.failing_read(&id(3), CloudSyncError::Unavailable { status: 503, code: "unknown".into() });

    let report = run(&world, &cloud).expect("run");

    let (_, uploaded, pulled) = ids(&report);
    assert_eq!(uploaded, [id(1), id(4)], "what went up stays up");
    assert!(pulled.is_empty());
    assert_eq!(report.pull.stopped.as_ref().map(|(doc, _)| doc.clone()), Some(id(3)));
    assert_eq!(report.pull.not_tried, [id(9)]);
    assert_eq!(accounted(&report), 5);
}

#[test]
fn a_run_that_could_not_compare_has_sent_and_written_nothing() {
    let (world, cloud) = five_documents();
    let cloud = cloud.failing_list(CloudSyncError::NotSignedIn);

    let err = run(&world, &cloud).unwrap_err();

    assert_eq!(err, DocSyncError::Cloud(CloudSyncError::NotSignedIn));
    assert_eq!(cloud.calls(), [Call::List]);
    assert_eq!(world.row_of(3).expect("row").version, 1);
    assert!(world.row_of(9).is_none());
}

#[test]
fn a_run_with_nothing_on_either_side_reports_nothing() {
    let world = World::new();

    let report = run(&world, &FakeCloud::new()).expect("run");

    assert_eq!(report, SyncReport::default());
}

fn stop_with(error: CloudSyncError) -> Option<(String, DocSyncError)> {
    Some(("d".to_string(), DocSyncError::Cloud(error)))
}

#[test]
fn a_report_tells_a_cancel_from_an_error_that_stopped_the_run() {
    let net = || CloudSyncError::Network("down".into());
    // (name, push stop, pull stop, cancelled?)
    let cases = [
        ("no stop", None, None, false),
        ("uploads cancelled", stop_with(CloudSyncError::Cancelled), None, true),
        ("pulls cancelled", None, stop_with(CloudSyncError::Cancelled), true),
        ("uploads stopped by the network", stop_with(net()), None, false),
        ("pulls stopped by the network", None, stop_with(net()), false),
    ];
    for (name, push, pull, cancelled) in cases {
        let mut report = SyncReport::default();
        report.push.stopped = push;
        report.pull.stopped = pull;

        assert_eq!(report.was_cancelled(), cancelled, "{name}");
        assert_eq!(report.stop().is_some(), name != "no stop", "{name}");
    }
}

#[test]
fn a_cancelled_run_counts_the_document_it_stopped_on_as_not_tried_and_a_failed_one_does_not() {
    // (name, push stop, push not tried, pull stop, pull not tried, documents not tried)
    let cases = [
        ("done", None, 0, None, 0, 0),
        ("cancelled in the uploads", stop_with(CloudSyncError::Cancelled), 2, None, 3, 2 + 3 + 1),
        ("cancelled in the pulls", None, 0, stop_with(CloudSyncError::Cancelled), 4, 4 + 1),
        ("network stop in the uploads", stop_with(CloudSyncError::Network("x".into())), 2, None, 3, 2 + 3),
        ("network stop in the pulls", None, 0, stop_with(CloudSyncError::Network("x".into())), 4, 4),
    ];
    for (name, push_stop, push_not_tried, pull_stop, pull_not_tried, expected) in cases {
        let mut report = SyncReport::default();
        report.push.stopped = push_stop;
        report.push.not_tried = (0..push_not_tried).map(|n| format!("u{n}")).collect();
        report.pull.stopped = pull_stop;
        report.pull.not_tried = (0..pull_not_tried).map(|n| format!("p{n}")).collect();

        assert_eq!(report.not_tried(), expected, "{name}");
    }
}
