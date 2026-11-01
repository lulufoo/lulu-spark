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

fn sync_lines(cloud: &FakeCloud) -> (Value, Vec<journal::Line>) {
    sync_and_journal(&SyncGate::new(), cloud)
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
