use std::sync::Mutex;

use serde_json::json;

use super::*;
use crate::services::cloud_sync::{CloudDoc, CloudDocs, CloudSyncError};
use crate::services::doc_sync::cancel::Cancellable;
use crate::services::doc_sync::tests::{id, FakeCloud, World};
use crate::services::doc_sync::{compare, run_sync, DocSyncError, SyncGate};

/// This machine and the cloud, set up so that one run has something of every kind to do:
///
/// | document | here | cloud | action |
/// |---|---|---|---|
/// | 1 | v5 | v2 | upload |
/// | 2 | v3 | v3 | skip |
/// | 3 | v1 | v4 | pull, replacing the one here |
/// | 4 | v0 | none | upload |
/// | 9 | none | v7 | pull, a document new here |
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

fn statuses(snapshot: &Snapshot) -> Vec<Status> {
    snapshot.entries.iter().map(|entry| entry.status).collect()
}

fn entry_of(snapshot: &Snapshot, n: u32) -> Entry {
    snapshot.entries.iter().find(|entry| entry.doc_id == id(n)).unwrap_or_else(|| panic!("no entry for {n}")).clone()
}

/// A cloud that reads the record before each transfer, as a screen asking would, and then lets a
/// hook do what happens on this machine meanwhile.
struct Watching<'a> {
    inner: FakeCloud,
    progress: &'a SyncProgress,
    meanwhile: Box<dyn Fn(&str, &str) + 'a>,
    seen: Mutex<Vec<(String, String, Snapshot)>>,
}

impl<'a> Watching<'a> {
    fn new(inner: FakeCloud, progress: &'a SyncProgress) -> Self {
        Self { inner, progress, meanwhile: Box::new(|_, _| {}), seen: Mutex::new(Vec::new()) }
    }

    fn meanwhile(mut self, hook: impl Fn(&str, &str) + 'a) -> Self {
        self.meanwhile = Box::new(hook);
        self
    }

    fn look(&self, transfer: &str, doc_id: &str) {
        let snapshot = self.progress.snapshot();
        self.seen.lock().expect("seen").push((transfer.to_string(), doc_id.to_string(), snapshot));
        (self.meanwhile)(transfer, doc_id);
    }

    fn seen(&self) -> Vec<(String, String, Snapshot)> {
        self.seen.lock().expect("seen").clone()
    }
}

impl CloudDocs for Watching<'_> {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        self.inner.list_documents()
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        self.look("upload", document_id);
        self.inner.upload(document_id, package, version)
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.look("read", document_id);
        self.inner.read(document_id)
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        self.inner.delete(document_id)
    }
}

#[test]
fn the_plan_lists_every_upload_then_every_download_by_id_and_leaves_out_what_is_the_same() {
    let (world, cloud) = five_documents();
    let progress = SyncProgress::new();

    progress.plan(&world.conn, &compare(&world.conn, &cloud).expect("compare"));

    let snapshot = progress.snapshot();
    let listed: Vec<(String, Op)> = snapshot.entries.iter().map(|e| (e.doc_id.clone(), e.op)).collect();
    assert_eq!(listed, [(id(1), Op::Upload), (id(4), Op::Upload), (id(3), Op::Pull), (id(9), Op::Pull)]);
    assert!(snapshot.entries.iter().all(|e| e.status == Status::NotStarted && e.reason.is_none()));
}

#[test]
fn an_entry_is_named_by_title_then_file_name_then_short_id_and_knows_if_the_document_is_here() {
    let (world, cloud) = five_documents();
    world
        .conn
        .execute("UPDATE document SET title = 'Quarterly plan' WHERE doc_id = ?1", [id(1)])
        .expect("title");
    let progress = SyncProgress::new();

    progress.plan(&world.conn, &compare(&world.conn, &cloud).expect("compare"));

    let snapshot = progress.snapshot();
    let named: Vec<(String, bool)> = snapshot.entries.iter().map(|e| (e.name.clone(), e.local)).collect();
    assert_eq!(
        named,
        [
            ("Quarterly plan".to_string(), true),
            ("202606190004-topic.md".to_string(), true),
            ("202606190003-topic.md".to_string(), true),
            (id(9)[..8].to_string(), false),
        ],
        "a title, a file name, the file name of the document a download replaces, and the short id of a document new here"
    );
}

#[test]
fn a_name_falls_back_from_the_title_to_the_file_name_to_the_short_id() {
    let mut row = DocRow::new(&id(7), "notes/deep/meeting.md", "p");
    row.title = Some("  A title  ".to_string());
    assert_eq!(name_of(&row), "A title");

    row.title = Some("   ".to_string());
    assert_eq!(name_of(&row), "meeting.md", "a blank title is no title");

    row.title = None;
    assert_eq!(name_of(&row), "meeting.md");

    row.raw_path = "notes/".to_string();
    assert_eq!(name_of(&row), &id(7)[..8], "a path that ends in a folder names no file");
}

#[test]
fn reading_all_of_it_through_a_run_keeps_the_count_and_moves_each_document_on_in_order() {
    let (world, fake) = five_documents();
    let progress = SyncProgress::new();
    let cloud = Watching::new(fake, &progress);

    run_sync(&world.conn, &world.workspaces_root, &cloud, &progress).expect("run");

    use Status::{Done, NotStarted, Running};
    let seen = cloud.seen();
    let looked: Vec<(&str, &str)> = seen.iter().map(|(transfer, doc, _)| (transfer.as_str(), doc.as_str())).collect();
    let (one, four, three, nine) = (id(1), id(4), id(3), id(9));
    assert_eq!(looked, [("upload", &one[..]), ("upload", &four[..]), ("read", &three[..]), ("read", &nine[..])]);
    assert_eq!(statuses(&seen[0].2), [Running, NotStarted, NotStarted, NotStarted]);
    assert_eq!(statuses(&seen[1].2), [Done, Running, NotStarted, NotStarted]);
    assert_eq!(statuses(&seen[2].2), [Done, Done, Running, NotStarted]);
    assert_eq!(statuses(&seen[3].2), [Done, Done, Done, Running]);
    assert!(seen.iter().all(|(_, _, snapshot)| snapshot.entries.len() == 4), "the list never changes length");

    let before_it_came = entry_of(&seen[3].2, 9);
    assert_eq!((before_it_came.name.as_str(), before_it_came.local), (&id(9)[..8], false));
    let after = progress.snapshot();
    assert_eq!(statuses(&after), [Done, Done, Done, Done]);
    let came = entry_of(&after, 9);
    assert_eq!((came.name.as_str(), came.local), ("202606190009-topic.md", true), "named by its file once it is here");
}

#[test]
fn a_document_that_cannot_be_packed_fails_with_its_reason_and_the_next_one_goes_on() {
    let world = World::new();
    world.add_without_file(1, 1);
    world.add(2, 1);
    let progress = SyncProgress::new();

    run_sync(&world.conn, &world.workspaces_root, &FakeCloud::new(), &progress).expect("run");

    let snapshot = progress.snapshot();
    assert_eq!(statuses(&snapshot), [Status::Failed, Status::Done]);
    let reason = entry_of(&snapshot, 1).reason.expect("a reason");
    assert!(reason.starts_with(&format!("could not pack document {}", id(1))), "{reason}");
    assert_eq!(entry_of(&snapshot, 2).reason, None);
}

#[test]
fn the_document_an_error_stopped_the_run_on_fails_and_the_ones_after_it_are_not_started() {
    let world = World::new();
    world.add(1, 1);
    world.add(2, 1);
    world.add(3, 1);
    let cloud = FakeCloud::new().with(&id(9), 3).failing_upload(&id(2), CloudSyncError::Network("down".into()));
    let progress = SyncProgress::new();

    let report = run_sync(&world.conn, &world.workspaces_root, &cloud, &progress).expect("run");

    assert!(report.stop().is_some());
    let snapshot = progress.snapshot();
    assert_eq!(statuses(&snapshot), [Status::Done, Status::Failed, Status::NotStarted, Status::NotStarted]);
    let why = DocSyncError::Cloud(CloudSyncError::Network("down".into())).to_string();
    assert_eq!(entry_of(&snapshot, 2).reason, Some(why));
    assert_eq!(entry_of(&snapshot, 9).op, Op::Pull, "the download was never tried");
}

#[test]
fn a_cancel_leaves_what_was_not_sent_not_started_and_so_is_the_transfer_it_refused() {
    struct CancelAfterFirstUpload<'a> {
        inner: FakeCloud,
        gate: &'a SyncGate,
    }
    impl CloudDocs for CancelAfterFirstUpload<'_> {
        fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
            self.inner.list_documents()
        }
        fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
            let sent = self.inner.upload(document_id, package, version);
            self.gate.cancel();
            sent
        }
        fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
            self.inner.read(document_id)
        }
        fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
            self.inner.delete(document_id)
        }
    }
    let (world, fake) = five_documents();
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("begin");
    let asking = CancelAfterFirstUpload { inner: fake, gate: &gate };
    let cloud = Cancellable::new(&asking, &run);

    let report = run_sync(&world.conn, &world.workspaces_root, &cloud, gate.progress()).expect("run");

    assert!(report.was_cancelled());
    let snapshot = gate.progress().snapshot();
    assert_eq!(statuses(&snapshot), [Status::Done, Status::NotStarted, Status::NotStarted, Status::NotStarted]);
    assert!(snapshot.entries.iter().all(|entry| entry.reason.is_none()), "a cancel is nobody's failure");
}

#[test]
fn a_document_deleted_before_its_turn_or_caught_up_during_its_download_leaves_the_list() {
    let (world, fake) = five_documents();
    let progress = SyncProgress::new();
    let cloud = Watching::new(fake, &progress).meanwhile(|transfer, doc| {
        if transfer == "upload" && doc == id(1) {
            world.remove_row(4);
        }
        if transfer == "read" && doc == id(3) {
            world.set_version(3, 9);
        }
    });

    let report = run_sync(&world.conn, &world.workspaces_root, &cloud, &progress).expect("run");

    assert_eq!((report.push.gone.len(), report.pull.kept.len()), (1, 1));
    let left: Vec<String> = progress.snapshot().entries.iter().map(|e| e.doc_id.clone()).collect();
    assert_eq!(left, [id(1), id(9)], "what went across, and nothing that did not");
    assert_eq!(statuses(&progress.snapshot()), [Status::Done, Status::Done]);
}

#[test]
fn a_run_that_could_not_compare_leaves_the_list_empty() {
    let world = World::new();
    world.add(1, 1);
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("begin");
    let cloud = FakeCloud::new().failing_list(CloudSyncError::NotSignedIn);

    assert!(run_sync(&world.conn, &world.workspaces_root, &cloud, gate.progress()).is_err());

    drop(run);
    assert_eq!(gate.progress().snapshot(), Snapshot { running: false, entries: Vec::new() });
}

#[test]
fn the_gate_clears_the_record_when_a_run_begins_and_closes_it_when_the_run_ends() {
    let (world, cloud) = five_documents();
    let gate = SyncGate::new();
    assert_eq!(gate.progress().snapshot(), Snapshot { running: false, entries: Vec::new() });

    let first = gate.try_begin().expect("begin");
    assert!(gate.progress().snapshot().running);
    run_sync(&world.conn, &world.workspaces_root, &cloud, gate.progress()).expect("run");
    assert!(gate.try_begin().is_none(), "a second run is refused");
    let during = gate.progress().snapshot();
    assert!(during.running);
    assert_eq!(during.entries.len(), 4, "and the refusal leaves the record of the run that is going on");

    drop(first);
    let after = gate.progress().snapshot();
    assert!(!after.running);
    assert_eq!(statuses(&after), [Status::Done; 4], "what the run wrote stays for the screen to read");

    let second = gate.try_begin().expect("begin again");
    let fresh = gate.progress().snapshot();
    assert_eq!(fresh, Snapshot { running: true, entries: Vec::new() }, "the next run begins with nothing");
    drop(second);
}

#[test]
fn a_run_that_panics_still_closes_its_record() {
    let gate = SyncGate::new();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _run = gate.try_begin().expect("begin");
        panic!("the run broke");
    }));

    assert!(outcome.is_err());
    assert!(!gate.progress().snapshot().running);
    assert!(gate.try_begin().is_some(), "and the gate is open");
}

#[test]
fn the_snapshot_reads_as_one_whole_list_in_the_words_the_screen_uses() {
    let world = World::new();
    world.add(1, 1);
    let cloud = FakeCloud::new().with(&id(9), 3);
    let progress = SyncProgress::new();
    progress.plan(&world.conn, &compare(&world.conn, &cloud).expect("compare"));
    progress.start(&id(1));
    progress.fail(&id(1), &DocSyncError::Pack { doc_id: id(1), message: "a file is missing".to_string() });

    assert_eq!(
        progress.snapshot().to_json(),
        json!({
            "ok": true,
            "running": false,
            "entries": [
                {
                    "id": id(1),
                    "name": "202606190001-topic.md",
                    "op": "upload",
                    "state": "failed",
                    "reason": format!("could not pack document {}: a file is missing", id(1)),
                    "local": true,
                },
                {
                    "id": id(9),
                    "name": &id(9)[..8],
                    "op": "pull",
                    "state": "not_started",
                    "reason": null,
                    "local": false,
                },
            ],
        })
    );
}

#[test]
fn a_change_to_a_document_the_record_does_not_know_is_ignored() {
    let progress = SyncProgress::new();

    progress.start(&id(1));
    progress.done(&id(1));
    progress.fail(&id(1), &DocSyncError::Database("x".to_string()));
    progress.forget(&id(1));

    assert_eq!(progress.snapshot(), Snapshot { running: false, entries: Vec::new() });
}
