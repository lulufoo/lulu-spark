use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};

use super::delete_doc_with;
use crate::services::cloud_sync::{CloudDoc, CloudDocs, CloudSyncError};
use crate::services::docs::{create_doc, create_doc_content, delete_doc};
use crate::services::spark_data::{find_doc, open};
use crate::test_support::TestSandbox;

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30

---

摘要正文，enough content.
"#;

const PNG: &[u8] = b"\x89PNG\r\n";

fn setup_notes() -> TestSandbox {
    let sandbox = TestSandbox::new();
    let notes = crate::test_support::default_workspace_dir();
    fs::create_dir_all(notes.join("raw")).expect("raw dir");
    fs::create_dir_all(notes.join("digest")).expect("digest dir");
    sandbox
}

fn stage_bytes(sandbox: &TestSandbox, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&dir).expect("stage dir");
    let p = dir.join(name);
    fs::write(&p, bytes).expect("write");
    p.canonicalize().expect("canon")
}

fn theme_dir() -> PathBuf {
    crate::test_support::default_workspace_dir()
        .join("raw")
        .join(crate::test_support::default_inbox_id())
        .join("notes")
}

#[test]
fn delete_doc_removes_the_attached_dir() {
    let sandbox = setup_notes();
    let source = stage_bytes(&sandbox, "source.md", SAMPLE_DOC.as_bytes());
    let png = stage_bytes(&sandbox, "overview-diagram.png", PNG);
    let created = create_doc(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "source_path": source.to_str().unwrap(),
            "title": "Test Title",
            "digest": "never",
            "source_type": "summary",
            "asset_paths": [png.to_str().unwrap()],
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    let attached = theme_dir().join(".attached").join(id);
    assert!(attached.join("overview-diagram.png").is_file());
    let deleted = delete_doc(&json!({ "id": id }));
    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
    assert!(!attached.exists());
    assert!(!theme_dir().join(".attached").exists());
}

#[test]
fn delete_doc_without_an_attached_dir_still_succeeds() {
    let _sandbox = setup_notes();
    let created = create_doc_content(&json!({
            "workspace_id": crate::test_support::default_workspace().id,
            "title": "T",
            "content": "# 标题\n\n这是正文，足够写入。\n",
            "digest": "never",
            "source_type": "summary",
        }),
    );
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    let id = created["id"].as_str().unwrap();
    assert!(!theme_dir().join(".attached").join(id).exists());
    let deleted = delete_doc(&json!({ "id": id }));
    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
}

// --- the cloud copy ---

/// What the document looked like on this machine at the moment the cloud was asked to delete it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AskedWhen {
    doc_id: String,
    row_there: bool,
    files_there: usize,
}

/// A cloud that only answers `delete`, with `answer`. Each ask is recorded together with what was
/// still on this machine then, so the order (cloud first) can be told.
struct DeleteSpy {
    answer: Result<(), CloudSyncError>,
    asked: Mutex<Vec<AskedWhen>>,
}

impl DeleteSpy {
    fn answering(answer: Result<(), CloudSyncError>) -> Self {
        Self { answer, asked: Mutex::new(Vec::new()) }
    }

    fn asked(&self) -> Vec<AskedWhen> {
        self.asked.lock().unwrap().clone()
    }
}

impl CloudDocs for DeleteSpy {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        unreachable!("deleting a document does not list the cloud")
    }

    fn upload(&self, _: &str, _: &[u8], _: i64) -> Result<(), CloudSyncError> {
        unreachable!("deleting a document does not upload")
    }

    fn read(&self, _: &str) -> Result<Vec<u8>, CloudSyncError> {
        unreachable!("deleting a document does not read the cloud")
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        let row_there = open().ok().and_then(|conn| find_doc(&conn, document_id).ok().flatten()).is_some();
        self.asked.lock().unwrap().push(AskedWhen {
            doc_id: document_id.to_string(),
            row_there,
            files_there: files_under(&theme_dir()).len(),
        });
        self.answer.clone()
    }
}

/// Every regular file under `dir`, attachments included.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// A note with one attachment; returns its id.
fn new_note(sandbox: &TestSandbox) -> String {
    let source = stage_bytes(sandbox, "source.md", SAMPLE_DOC.as_bytes());
    let png = stage_bytes(sandbox, "overview-diagram.png", PNG);
    let created = create_doc(&json!({
        "workspace_id": crate::test_support::default_workspace().id,
        "source_path": source.to_str().unwrap(),
        "title": "Test Title",
        "digest": "never",
        "source_type": "summary",
        "asset_paths": [png.to_str().unwrap()],
    }));
    assert_eq!(created.get("ok"), Some(&json!(true)), "{created}");
    created["id"].as_str().unwrap().to_string()
}

fn row_exists(id: &str) -> bool {
    find_doc(&open().expect("db"), id).expect("query").is_some()
}

#[test]
fn a_signed_in_delete_removes_the_cloud_copy_first_and_then_the_document() {
    let sandbox = setup_notes();
    let id = new_note(&sandbox);
    let before = files_under(&theme_dir());
    assert!(before.len() >= 2, "a body and an attachment: {before:?}");
    let cloud = DeleteSpy::answering(Ok(()));

    let deleted = delete_doc_with(&json!({ "id": id }), &cloud);

    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
    assert_eq!(
        cloud.asked(),
        [AskedWhen { doc_id: id.clone(), row_there: true, files_there: before.len() }],
        "the cloud is asked once, while the whole document is still on this machine"
    );
    assert!(!row_exists(&id));
    assert_eq!(files_under(&theme_dir()), Vec::<PathBuf>::new());
}

#[test]
fn a_failed_cloud_delete_fails_the_whole_delete_and_keeps_the_document() {
    let failures = [
        CloudSyncError::Network("connection refused".into()),
        CloudSyncError::SessionExpired,
        CloudSyncError::Unauthorized,
        CloudSyncError::Unavailable { status: 503, code: "unknown".into() },
        CloudSyncError::Rejected { status: 403, code: "forbidden".into() },
        CloudSyncError::BadResponse("unexpected status 302".into()),
        CloudSyncError::Vault("locked".into()),
    ];
    for failure in failures {
        let sandbox = setup_notes();
        let id = new_note(&sandbox);
        let before = files_under(&theme_dir());
        let cloud = DeleteSpy::answering(Err(failure.clone()));

        let refused = delete_doc_with(&json!({ "id": id }), &cloud);

        assert_eq!(refused.get("ok"), None, "{failure}: {refused}");
        assert_eq!(refused["_status"], json!(502), "{failure}");
        let message = refused["error"].as_str().expect("an error message");
        assert!(message.contains("not deleted"), "{message}");
        assert!(message.contains(&failure.to_string()), "the cause is named: {message}");
        assert_eq!(cloud.asked().len(), 1, "{failure}");
        assert!(row_exists(&id), "{failure}: the row is kept");
        assert_eq!(files_under(&theme_dir()), before, "{failure}: the files are kept");

        // The document is still there to delete once the cloud answers.
        let retried = delete_doc_with(&json!({ "id": id }), &DeleteSpy::answering(Ok(())));
        assert_eq!(retried.get("ok"), Some(&json!(true)), "{failure}: {retried}");
        assert!(!row_exists(&id));
    }
}

#[test]
fn a_user_who_is_not_signed_in_deletes_on_this_machine_only() {
    let sandbox = setup_notes();
    let id = new_note(&sandbox);
    let cloud = DeleteSpy::answering(Err(CloudSyncError::NotSignedIn));

    let deleted = delete_doc_with(&json!({ "id": id }), &cloud);

    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
    assert!(!row_exists(&id));
    assert_eq!(files_under(&theme_dir()), Vec::<PathBuf>::new());
}

/// The file of a document can be gone while its record stays (a clean-up on the disk, a sync that
/// never brought it). The page then cannot open it, and the delete is the way out: it must not need
/// the file.
#[test]
fn a_document_whose_files_are_gone_is_still_deleted() {
    let sandbox = setup_notes();
    let id = new_note(&sandbox);
    for file in files_under(&theme_dir()) {
        fs::remove_file(&file).expect("remove a file of the document");
    }
    assert_eq!(files_under(&theme_dir()), Vec::<PathBuf>::new());
    assert!(row_exists(&id), "the record is still there");
    let cloud = DeleteSpy::answering(Ok(()));

    let deleted = delete_doc_with(&json!({ "id": id }), &cloud);

    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
    assert_eq!(deleted["removed"], json!(0), "there was no file to remove: {deleted}");
    assert!(!row_exists(&id));
    assert_eq!(
        cloud.asked(),
        [AskedWhen { doc_id: id.clone(), row_there: true, files_there: 0 }],
        "the cloud is still asked first, once"
    );
}

#[test]
fn an_id_the_cloud_cannot_hold_is_deleted_on_this_machine() {
    let sandbox = setup_notes();
    let id = new_note(&sandbox);
    let cloud = DeleteSpy::answering(Err(CloudSyncError::InvalidInput("not a document id".into())));

    let deleted = delete_doc_with(&json!({ "id": id }), &cloud);

    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
    assert!(!row_exists(&id));
}

#[test]
fn a_delete_refused_before_it_starts_never_reaches_the_cloud() {
    let sandbox = setup_notes();
    let id = new_note(&sandbox);
    let cloud = DeleteSpy::answering(Ok(()));
    let unknown = "f".repeat(32);

    let refusals = [
        (json!({}), 400),
        (json!({ "id": unknown }), 404),
        (json!({ "id": id, "workspace_id": crate::test_support::default_workspace().id }), 400),
    ];
    for (payload, status) in refusals {
        let refused = delete_doc_with(&payload, &cloud);
        assert_eq!(refused["_status"], json!(status), "{payload}: {refused}");
    }

    // A row whose body path is not one a delete will touch.
    open()
        .expect("db")
        .execute("UPDATE document SET raw_path = '../elsewhere.md' WHERE doc_id = ?1", [&id])
        .expect("break the path");
    let refused: Value = delete_doc_with(&json!({ "id": id }), &cloud);
    assert_eq!(refused["_status"], json!(400), "{refused}");

    assert_eq!(cloud.asked(), Vec::<AskedWhen>::new());
    assert!(row_exists(&id));
}

#[test]
fn with_nobody_signed_in_the_app_deletes_on_this_machine_without_asking_anyone() {
    // `delete_doc` is the app's own: the cloud is the real one, and the vault of a test holds no
    // session. This also keeps a unit test from ever reaching the network.
    let sandbox = setup_notes();
    let id = new_note(&sandbox);

    let deleted = delete_doc(&json!({ "id": id }));

    assert_eq!(deleted.get("ok"), Some(&json!(true)), "{deleted}");
    assert!(!row_exists(&id));
}
