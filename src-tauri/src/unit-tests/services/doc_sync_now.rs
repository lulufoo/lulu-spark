use std::sync::Mutex;

use serde_json::{json, Value};

use super::*;
use crate::services::cloud_sync::{CloudDoc, CloudDocs, CloudSyncError};
use crate::services::doc_sync::tests::{id, Call, FakeCloud};
use crate::services::doc_zip::pack_doc;
use crate::services::docs::{create_doc_content, create_workspace, delete_doc, workspace_dir};
use crate::services::spark_data;
use crate::services::app_log::Level;
use crate::services::search_index::{open as open_index, search_in_cache, SearchFilter};
use crate::test_support::TestSandbox;

const WORD: &str = "角色扮演";

fn new_doc(title: &str, body: &str) -> String {
    let created = create_doc_content(&json!({
        "workspace_id": crate::test_support::default_workspace().id,
        "title": title,
        "content": format!("# {title}\n\n{body}\n"),
        "digest": "never",
        "source_type": "summary",
    }));
    created["id"].as_str().unwrap_or_else(|| panic!("create failed: {created}")).to_string()
}

fn sync(cloud: &FakeCloud) -> Value {
    sync_with(&SyncGate::new(), cloud)
}

/// The answer, the lines written as the run went (`live`), and the lines written when it ended.
fn sync_both(cloud: &dyn CloudDocs) -> (Value, Vec<journal::Line>, Vec<journal::Line>) {
    let live = Mutex::new(Vec::new());
    let (answer, ended) =
        sync_and_journal(&SyncGate::new(), cloud, &|line| live.lock().expect("live").push(line));
    (answer, live.into_inner().expect("live"), ended)
}

/// The answer and the lines written when the run ended.
fn sync_lines(cloud: &FakeCloud) -> (Value, Vec<journal::Line>) {
    let (answer, _, ended) = sync_both(cloud);
    (answer, ended)
}

fn events(lines: &[journal::Line]) -> Vec<&'static str> {
    lines.iter().map(|l| l.event).collect()
}

/// A cloud that asks the gate, at every call, whether the run is still holding it.
struct GateProbe<'a> {
    inner: FakeCloud,
    gate: &'a SyncGate,
    refused: Mutex<Vec<bool>>,
}

impl GateProbe<'_> {
    fn look(&self) {
        self.refused.lock().expect("probe").push(self.gate.try_begin().is_none());
    }
}

impl CloudDocs for GateProbe<'_> {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        self.look();
        self.inner.list_documents()
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        self.look();
        self.inner.upload(document_id, package, version)
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.look();
        self.inner.read(document_id)
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        self.look();
        self.inner.delete(document_id)
    }
}

fn pulled(workspace: &str, doc: &str) -> PulledDoc {
    PulledDoc { doc_id: doc.into(), version: 1, bytes: 1, created: false, workspace_id: workspace.into() }
}

#[test]
fn the_documents_pulled_are_re_indexed_workspace_by_workspace() {
    let batches = index_batches(&[
        pulled("w2", "d3"),
        pulled("w1", "d1"),
        pulled("w2", "d2"),
        pulled("w1", "d4"),
    ]);

    assert_eq!(
        batches,
        [
            ("w1".to_string(), vec!["d1".to_string(), "d4".to_string()]),
            ("w2".to_string(), vec!["d3".to_string(), "d2".to_string()]),
        ]
    );
}

#[test]
fn nothing_pulled_means_nothing_to_re_index() {
    assert_eq!(index_batches(&[]), Vec::<(String, Vec<String>)>::new());
}

#[test]
fn a_sync_sends_a_new_document_and_reports_the_counts() {
    let _sandbox = TestSandbox::new();
    let doc = new_doc("First", "这是一段足够长的正文。");
    let cloud = FakeCloud::new();

    let answer = sync(&cloud);

    assert_eq!(answer["ok"], json!(true), "{answer}");
    assert_eq!((answer["uploaded"].clone(), answer["pulled"].clone(), answer["skipped"].clone()), (json!(1), json!(0), json!(0)));
    assert_eq!(answer["failed"], json!([]));
    assert_eq!(answer["stopped"], json!(null));
    assert_eq!(cloud.stored(&doc).expect("in the cloud").0, 0);
}

#[test]
fn a_second_sync_has_nothing_left_to_send() {
    let _sandbox = TestSandbox::new();
    new_doc("First", "这是一段足够长的正文。");
    let cloud = FakeCloud::new();
    sync(&cloud);

    let answer = sync(&cloud);

    assert_eq!((answer["uploaded"].clone(), answer["pulled"].clone(), answer["skipped"].clone()), (json!(0), json!(0), json!(1)));
}

#[test]
fn a_document_missing_here_comes_back_and_can_be_searched() {
    let sandbox = TestSandbox::new();
    let doc = new_doc("Roles", &format!("这篇讲{WORD}的做法。"));
    let cloud = FakeCloud::new();
    sync(&cloud);
    let removed = delete_doc(&json!({ "id": doc }));
    assert_eq!(removed["ok"], json!(true), "{removed}");
    let cache = sandbox.cache_dir();
    drop(open_index(&cache).expect("create index"));
    assert!(search_in_cache(&cache, WORD, &SearchFilter::default(), 10).expect("before").is_empty());

    let answer = sync(&cloud);

    assert_eq!(answer["pulled"], json!(1), "{answer}");
    let found = search_in_cache(&cache, WORD, &SearchFilter::default(), 10).expect("after");
    assert_eq!(found.iter().map(|h| h.doc_id.as_str()).collect::<Vec<_>>(), [doc.as_str()], "the index follows the pull");
}

/// A document in a workspace of its own, written while no search index exists.
fn doc_in_new_workspace(name: &str, body: &str) -> (String, String) {
    let made = create_workspace(&json!({ "workspace_name": name }));
    let workspace = made["workspace_id"].as_str().expect("workspace id").to_string();
    let created = create_doc_content(&json!({
        "workspace_id": workspace, "title": name, "content": format!("# {name}\n\n{body}\n"),
        "digest": "never", "source_type": "summary",
    }));
    let doc = created["id"].as_str().unwrap_or_else(|| panic!("create failed: {created}")).to_string();
    (workspace, doc)
}

#[test]
fn documents_pulled_into_two_workspaces_are_all_searchable() {
    let sandbox = TestSandbox::new();
    let (w1, d1) = doc_in_new_workspace("One", "这篇讲角色扮演的做法。");
    let (w2, d2) = doc_in_new_workspace("Two", "这篇讲知识蒸馏的做法。");
    let root = crate::config::paths::workspaces_root().expect("workspaces root");
    let conn = spark_data::open().expect("db");
    let package = |ws: &str, doc: &str| {
        let row = spark_data::get_doc(&conn, doc).expect("query").expect("row");
        pack_doc(&workspace_dir(&root, ws).expect("workspace folder"), &row).expect("pack")
    };
    // The cloud holds a newer version of both, so the sync pulls over them where they are.
    let cloud = FakeCloud::new().with_package(&d1, 1, package(&w1, &d1)).with_package(&d2, 1, package(&w2, &d2));
    let cache = sandbox.cache_dir();
    drop(open_index(&cache).expect("create index"));
    let search = |word: &str| search_in_cache(&cache, word, &SearchFilter::default(), 10).expect("search");
    assert!(search("角色扮演").is_empty() && search("知识蒸馏").is_empty(), "nothing is indexed yet");

    let answer = sync(&cloud);

    assert_eq!(answer["pulled"], json!(2), "{answer}");
    let ids = |hits: Vec<crate::services::search_index::QueryHit>| hits.into_iter().map(|h| h.doc_id).collect::<Vec<_>>();
    assert_eq!(ids(search("角色扮演")), [d1], "workspace one is indexed");
    assert_eq!(ids(search("知识蒸馏")), [d2], "workspace two is indexed");
}

#[test]
fn nobody_signed_in_is_a_clear_error_and_nothing_changes() {
    let _sandbox = TestSandbox::new();
    let doc = new_doc("First", "这是一段足够长的正文。");
    let cloud = FakeCloud::new().failing_list(CloudSyncError::NotSignedIn);

    let (answer, lines) = sync_lines(&cloud);

    assert_eq!(answer, json!({ "error": "Cannot sync: not signed in", "_status": 401 }));
    assert_eq!(cloud.calls(), [Call::List], "nothing was sent");
    assert!(spark_data::find_doc(&spark_data::open().expect("db"), &doc).expect("query").is_some());
    assert_eq!(
        lines,
        [journal::Line { level: Level::Warn, event: "sync.not_made", params: json!({ "error": "not signed in" }) }]
    );
}

#[test]
fn a_run_that_goes_on_while_another_runs_is_refused_and_does_not_touch_the_cloud() {
    let _sandbox = TestSandbox::new();
    let gate = SyncGate::new();
    let cloud = FakeCloud::new();
    let held = gate.try_begin().expect("first run");

    let answer = sync_with(&gate, &cloud);

    assert_eq!(answer, json!({ "error": "Cannot sync: a sync is already running", "_status": 409 }));
    assert_eq!(cloud.calls(), Vec::<Call>::new());
    drop(held);
    assert_eq!(sync_with(&gate, &cloud)["ok"], json!(true), "the gate opens when the run ends");
}

#[test]
fn the_gate_is_held_from_the_first_call_to_the_cloud_to_the_last() {
    let _sandbox = TestSandbox::new();
    new_doc("First", "这是一段足够长的正文。");
    let gate = SyncGate::new();
    let probe = GateProbe { inner: FakeCloud::new(), gate: &gate, refused: Mutex::new(Vec::new()) };

    let answer = sync_with(&gate, &probe);

    assert_eq!(answer["uploaded"], json!(1), "{answer}");
    assert_eq!(*probe.refused.lock().expect("probe"), [true, true], "a list and an upload, both under the gate");
}

#[test]
fn the_gate_is_open_again_after_a_run_that_failed() {
    let _sandbox = TestSandbox::new();
    let gate = SyncGate::new();
    let failing = FakeCloud::new().failing_list(CloudSyncError::Network("down".into()));

    assert_eq!(sync_with(&gate, &failing)["_status"], json!(502));

    assert_eq!(sync_with(&gate, &FakeCloud::new())["ok"], json!(true));
}

#[test]
fn a_run_writes_a_line_for_each_step_as_it_goes() {
    let _sandbox = TestSandbox::new();
    let gone = new_doc("Gone", "这是一段足够长的正文。");
    let cloud = FakeCloud::new();
    sync(&cloud);
    assert_eq!(delete_doc(&json!({ "id": gone }))["ok"], json!(true));
    let sent = new_doc("Sent", "另一段足够长的正文。");

    let (answer, live, ended) = sync_both(&cloud);

    assert_eq!((answer["uploaded"].clone(), answer["pulled"].clone()), (json!(1), json!(1)), "{answer}");
    assert_eq!(events(&live), ["sync.started", "sync.listed", "sync.doc_uploaded", "sync.doc_downloaded"]);
    assert_eq!(live[1].params["cloud_docs"], json!(1));
    assert_eq!(live[2].params["doc_id"], json!(sent));
    assert_eq!(live[3].params["doc_id"], json!(gone));
    assert_eq!(events(&ended), ["sync.finished"], "the line of the end is still written when it ends");
}

#[test]
fn a_run_that_could_not_list_the_cloud_only_says_it_began() {
    let _sandbox = TestSandbox::new();
    new_doc("First", "这是一段足够长的正文。");
    let cloud = FakeCloud::new().failing_list(CloudSyncError::Network("down".into()));

    let (_, live, ended) = sync_both(&cloud);

    assert_eq!(events(&live), ["sync.started"]);
    assert_eq!(events(&ended), ["sync.not_made"]);
}

#[test]
fn a_run_that_is_refused_writes_no_line_at_all() {
    let _sandbox = TestSandbox::new();
    let gate = SyncGate::new();
    let held = gate.try_begin().expect("first run");
    let live = Mutex::new(Vec::new());

    let (answer, ended) =
        sync_and_journal(&gate, &FakeCloud::new(), &|line| live.lock().expect("live").push(line));

    assert_eq!(answer["_status"], json!(409));
    assert!(live.lock().expect("live").is_empty());
    assert!(ended.is_empty());
    drop(held);
}

#[test]
fn a_document_that_failed_is_not_logged_as_sent() {
    let _sandbox = TestSandbox::new();
    let bad = new_doc("Too big", "这是一段足够长的正文。");
    let good = new_doc("Fine", "另一段足够长的正文。");
    let cloud = FakeCloud::new().failing_upload(&bad, CloudSyncError::QuotaExceeded { limit_bytes: 100, used_bytes: 100 });

    let (_, live, ended) = sync_both(&cloud);

    assert_eq!(events(&live), ["sync.started", "sync.listed", "sync.doc_uploaded"]);
    assert_eq!(live[2].params["doc_id"], json!(good));
    assert_eq!(events(&ended), ["sync.doc_failed", "sync.finished"]);
}

#[test]
fn a_document_that_could_not_be_sent_is_reported_and_logged_with_its_reason() {
    let _sandbox = TestSandbox::new();
    let bad = new_doc("Too big", "这是一段足够长的正文。");
    let good = new_doc("Fine", "另一段足够长的正文。");
    let cloud = FakeCloud::new().failing_upload(&bad, CloudSyncError::QuotaExceeded { limit_bytes: 100, used_bytes: 100 });

    let (answer, lines) = sync_lines(&cloud);

    assert_eq!(answer["ok"], json!(true), "the run was made: {answer}");
    assert_eq!(answer["uploaded"], json!(1));
    assert_eq!(answer["failed"].as_array().map(Vec::len), Some(1));
    assert_eq!(answer["failed"][0]["id"], json!(bad));
    assert_eq!(answer["failed"][0]["op"], json!("upload"));
    assert!(cloud.stored(&good).is_some());
    assert_eq!(events(&lines), ["sync.doc_failed", "sync.finished"]);
    assert_eq!(lines[0].params["doc_id"], json!(bad));
    assert_eq!(lines[0].params["error"], json!("the cloud storage limit is reached (100 of 100 bytes used)"));
}

#[test]
fn a_run_that_stopped_says_where_in_the_answer_and_in_the_log() {
    let _sandbox = TestSandbox::new();
    let doc = new_doc("First", "这是一段足够长的正文。");
    let cloud = FakeCloud::new().failing_upload(&doc, CloudSyncError::Network("down".into()));
    let elsewhere = id(9);
    let cloud = cloud.with(&elsewhere, 3);

    let (answer, lines) = sync_lines(&cloud);

    assert_eq!(answer["ok"], json!(true), "{answer}");
    assert_eq!(answer["stopped"]["id"], json!(doc));
    assert_eq!(answer["stopped"]["op"], json!("upload"));
    assert_eq!(answer["not_tried"], json!(1), "the pull of the other document was left for the next run");
    assert_eq!(events(&lines), ["sync.stopped", "sync.finished"]);
}

/// A cloud that asks the gate to cancel right after the `n`th transfer (upload or read) went
/// through, as the user would while that transfer is going on.
struct CancelAfter<'a> {
    inner: FakeCloud,
    gate: &'a SyncGate,
    transfers_left: Mutex<usize>,
    asked: Mutex<Vec<bool>>,
}

impl<'a> CancelAfter<'a> {
    fn new(inner: FakeCloud, gate: &'a SyncGate, transfers: usize) -> Self {
        Self { inner, gate, transfers_left: Mutex::new(transfers), asked: Mutex::new(Vec::new()) }
    }

    fn one_transfer_done(&self) {
        let mut left = self.transfers_left.lock().expect("left");
        *left = left.saturating_sub(1);
        if *left == 0 {
            self.asked.lock().expect("asked").push(self.gate.cancel());
        }
    }
}

impl CloudDocs for CancelAfter<'_> {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        self.inner.list_documents()
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        let result = self.inner.upload(document_id, package, version);
        self.one_transfer_done();
        result
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        let result = self.inner.read(document_id);
        self.one_transfer_done();
        result
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        self.inner.delete(document_id)
    }
}

#[test]
fn a_cancel_asked_during_an_upload_lets_that_upload_finish_and_stops_before_the_next() {
    let _sandbox = TestSandbox::new();
    let docs: Vec<String> = (0..3).map(|n| new_doc(&format!("Doc {n}"), "这是一段足够长的正文。")).collect();
    let gate = SyncGate::new();
    let cloud = CancelAfter::new(FakeCloud::new(), &gate, 1);

    let (answer, lines) = sync_and_journal(&gate, &cloud, &|_| {});

    assert_eq!(answer["ok"], json!(true), "a cancelled run was made: {answer}");
    assert_eq!(answer["cancelled"], json!(true));
    assert_eq!(answer["uploaded"], json!(1), "the document in flight was finished, not cut");
    assert_eq!(answer["not_tried"], json!(2), "the other two were left for the next run");
    assert_eq!(answer["stopped"], json!(null));
    assert_eq!(answer["failed"], json!([]));
    assert_eq!(*cloud.asked.lock().expect("asked"), [true], "the gate knew a run was going on");
    let stored = docs.iter().filter(|doc| cloud.inner.stored(doc).is_some()).count();
    assert_eq!(stored, 1, "exactly one document reached the cloud");
    assert_eq!(events(&lines), ["sync.cancelled", "sync.finished"]);
    assert_eq!(lines[0].level, Level::Info);
    assert_eq!(lines[1].params["cancelled"], json!(true));
}

#[test]
fn a_cancelled_run_is_finished_by_the_next_one() {
    let _sandbox = TestSandbox::new();
    let docs: Vec<String> = (0..3).map(|n| new_doc(&format!("Doc {n}"), "这是一段足够长的正文。")).collect();
    let gate = SyncGate::new();
    let cancelling = CancelAfter::new(FakeCloud::new(), &gate, 1);
    let first = sync_with(&gate, &cancelling);
    assert_eq!(first["cancelled"], json!(true));

    let second = sync_with(&gate, &cancelling.inner);

    assert_eq!(second["ok"], json!(true));
    assert_eq!(second["cancelled"], json!(false), "the cancel did not outlive its run");
    assert_eq!(second["uploaded"], json!(2), "the two documents the first run did not get to");
    assert_eq!(second["skipped"], json!(1), "the one it sent");
    assert!(docs.iter().all(|doc| cancelling.inner.stored(doc).is_some()));
}

#[test]
fn a_cancel_asked_during_a_pull_stops_the_pulls_and_brings_no_half_document() {
    let _sandbox = TestSandbox::new();
    let docs: Vec<String> = (0..2).map(|n| new_doc(&format!("Doc {n}"), "这是一段足够长的正文。")).collect();
    let fake = FakeCloud::new();
    assert_eq!(sync(&fake)["uploaded"], json!(2));
    for doc in &docs {
        assert_eq!(delete_doc(&json!({ "id": doc }))["ok"], json!(true));
    }
    let gate = SyncGate::new();
    let cloud = CancelAfter::new(fake, &gate, 1);

    let answer = sync_with(&gate, &cloud);

    assert_eq!(answer["cancelled"], json!(true), "{answer}");
    assert_eq!(answer["pulled"], json!(1), "the document being read was finished");
    assert_eq!(answer["not_tried"], json!(1), "the second one was never started");
    assert_eq!(answer["stopped"], json!(null));
    let conn = spark_data::open().expect("db");
    let here = docs.iter().filter(|doc| spark_data::find_doc(&conn, doc).expect("query").is_some()).count();
    assert_eq!(here, 1, "one document came back, the other did not");
}

#[test]
fn cancelling_with_no_sync_going_on_answers_so_and_does_not_hold_the_next_sync_back() {
    let _sandbox = TestSandbox::new();
    new_doc("First", "这是一段足够长的正文。");
    let gate = SyncGate::new();

    let answer = cancel_now(&gate);

    assert_eq!(answer, json!({ "ok": true, "cancelling": false }));
    let next = sync_with(&gate, &FakeCloud::new());
    assert_eq!(next["cancelled"], json!(false));
    assert_eq!(next["uploaded"], json!(1));
}

#[test]
fn cancelling_while_a_sync_goes_on_says_it_is_cancelling() {
    let _sandbox = TestSandbox::new();
    new_doc("First", "这是一段足够长的正文。");
    new_doc("Second", "另一段足够长的正文。");
    let gate = SyncGate::new();
    let cloud = CancelAfter::new(FakeCloud::new(), &gate, usize::MAX);
    let seen = Mutex::new(Vec::new());
    struct AskInTheList<'a> {
        inner: &'a CancelAfter<'a>,
        gate: &'a SyncGate,
        seen: &'a Mutex<Vec<Value>>,
    }
    impl CloudDocs for AskInTheList<'_> {
        fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
            self.seen.lock().expect("seen").push(cancel_now(self.gate));
            self.inner.list_documents()
        }
        fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
            self.inner.upload(document_id, package, version)
        }
        fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
            self.inner.read(document_id)
        }
        fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
            self.inner.delete(document_id)
        }
    }
    let asking = AskInTheList { inner: &cloud, gate: &gate, seen: &seen };

    let answer = sync_with(&gate, &asking);

    assert_eq!(*seen.lock().expect("seen"), [json!({ "ok": true, "cancelling": true })]);
    assert_eq!(answer["cancelled"], json!(true), "asked in the list, stopped at the first transfer: {answer}");
    assert_eq!(answer["uploaded"], json!(0));
    assert_eq!(answer["not_tried"], json!(2));
    assert_eq!(cloud.inner.calls(), [Call::List], "nothing was sent");
}

#[test]
fn a_cancel_that_comes_after_the_last_transfer_leaves_a_finished_run_finished() {
    let _sandbox = TestSandbox::new();
    new_doc("Only", "这是一段足够长的正文。");
    let gate = SyncGate::new();
    let cloud = CancelAfter::new(FakeCloud::new(), &gate, 1);

    let answer = sync_with(&gate, &cloud);

    assert_eq!(answer["uploaded"], json!(1));
    assert_eq!(answer["cancelled"], json!(false), "nothing was left to cancel");
    assert_eq!(answer["not_tried"], json!(0));
}
