//! Fixtures shared by the doc sync tests (a stand-in cloud, a temp database with its workspace
//! folders), and the comparison against the database.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::Connection;
use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::services::archive_parse::expected_lang_common_path;
use crate::services::cloud_sync::{CloudDoc, CloudSyncError};
use crate::services::doc_zip::pack_doc;
use crate::services::docs::attached::attached_dir;
use crate::services::docs::{doc_file_path, workspace_dir};
use crate::services::spark_data::{
    create_workspace, disable_workspace, get_default_project, get_default_workspace, insert_doc, insert_project,
    open_at, DocRow, ProjectRow, DB_FILE,
};

/// A valid document id, told apart by `n`.
pub(super) fn id(n: u32) -> String {
    format!("{n:032x}")
}

/// What the stand-in cloud was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Call {
    List,
    Upload { doc_id: String, version: i64, bytes: usize },
    Read { doc_id: String },
    Delete { doc_id: String },
}

#[derive(Default)]
struct CloudState {
    docs: BTreeMap<String, (i64, Vec<u8>)>,
    calls: Vec<Call>,
    list_error: Option<CloudSyncError>,
    upload_errors: HashMap<String, CloudSyncError>,
    read_errors: HashMap<String, CloudSyncError>,
}

/// A cloud in memory. It lists what it holds, keeps what is uploaded, and fails where told to.
#[derive(Default)]
pub(super) struct FakeCloud {
    state: Mutex<CloudState>,
}

impl FakeCloud {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// The cloud already holds this document at this version.
    pub(super) fn with(self, doc_id: &str, version: i64) -> Self {
        self.state.lock().unwrap().docs.insert(doc_id.to_string(), (version, b"old".to_vec()));
        self
    }

    /// The cloud holds this package for the document at this version.
    pub(super) fn with_package(self, doc_id: &str, version: i64, package: Vec<u8>) -> Self {
        self.state.lock().unwrap().docs.insert(doc_id.to_string(), (version, package));
        self
    }

    pub(super) fn failing_list(self, error: CloudSyncError) -> Self {
        self.state.lock().unwrap().list_error = Some(error);
        self
    }

    /// Every upload of this document fails with `error`.
    pub(super) fn failing_upload(self, doc_id: &str, error: CloudSyncError) -> Self {
        self.state.lock().unwrap().upload_errors.insert(doc_id.to_string(), error);
        self
    }

    /// Every read of this document fails with `error`.
    pub(super) fn failing_read(self, doc_id: &str, error: CloudSyncError) -> Self {
        self.state.lock().unwrap().read_errors.insert(doc_id.to_string(), error);
        self
    }

    pub(super) fn calls(&self) -> Vec<Call> {
        self.state.lock().unwrap().calls.clone()
    }

    pub(super) fn stored(&self, doc_id: &str) -> Option<(i64, Vec<u8>)> {
        self.state.lock().unwrap().docs.get(doc_id).cloned()
    }
}

impl CloudDocs for FakeCloud {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::List);
        if let Some(error) = &state.list_error {
            return Err(error.clone());
        }
        Ok(state
            .docs
            .iter()
            .map(|(document_id, (version, _))| CloudDoc { document_id: document_id.clone(), version: *version })
            .collect())
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Upload { doc_id: document_id.to_string(), version, bytes: package.len() });
        if let Some(error) = state.upload_errors.get(document_id) {
            return Err(error.clone());
        }
        state.docs.insert(document_id.to_string(), (version, package.to_vec()));
        Ok(())
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Read { doc_id: document_id.to_string() });
        if let Some(error) = state.read_errors.get(document_id) {
            return Err(error.clone());
        }
        state.docs.get(document_id).map(|(_, package)| package.clone()).ok_or(CloudSyncError::ObjectNotFound)
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        let mut state = self.state.lock().unwrap();
        state.calls.push(Call::Delete { doc_id: document_id.to_string() });
        state.docs.remove(document_id);
        Ok(())
    }
}

/// A temp folder holding a fresh database (its Default workspace and inbox) and the folder of
/// workspace folders. Nothing outside it is touched.
pub(super) struct World {
    _tmp: TempDir,
    pub(super) conn: Connection,
    pub(super) workspaces_root: PathBuf,
    pub(super) workspace_id: String,
    pub(super) project_id: String,
}

impl World {
    pub(super) fn new() -> Self {
        let tmp = TempDir::new().expect("temp dir");
        let conn = open_at(&tmp.path().join("data").join(DB_FILE)).expect("database");
        let workspace = get_default_workspace(&conn).expect("default workspace");
        let project = get_default_project(&conn, &workspace.id).expect("query").expect("inbox");
        Self {
            workspaces_root: tmp.path().join("workspaces"),
            _tmp: tmp,
            conn,
            workspace_id: workspace.id,
            project_id: project.id,
        }
    }

    /// A document in the Default workspace's inbox at `version`, with its body on disk.
    pub(super) fn add(&self, n: u32, version: i64) -> DocRow {
        self.add_in(&self.workspace_id.clone(), &self.project_id.clone(), n, version)
    }

    /// Like [`World::add`] for a document whose row exists but whose body file does not.
    pub(super) fn add_without_file(&self, n: u32, version: i64) -> DocRow {
        let row = self.row(&self.project_id, n, version);
        fs::create_dir_all(workspace_dir(&self.workspaces_root, &self.workspace_id).expect("folder")).expect("folder");
        insert_doc(&self.conn, &row).expect("insert");
        row
    }

    /// A document in a workspace whose id the database takes but no workspace folder can have
    /// (49 characters, one over the limit).
    pub(super) fn add_in_workspace_with_a_bad_id(&self, n: u32) -> DocRow {
        let workspace_id = "w".repeat(49);
        self.conn
            .execute("INSERT INTO workspace(id, name) VALUES (?1, 'Odd')", [&workspace_id])
            .expect("workspace");
        let project = ProjectRow::new("oddproj", &workspace_id, "Odd", "");
        insert_project(&self.conn, &project).expect("project");
        let row = self.row(&project.id, n, 1);
        insert_doc(&self.conn, &row).expect("insert");
        row
    }

    /// A document in a new workspace that is then disabled.
    pub(super) fn add_in_disabled_workspace(&self, n: u32, version: i64) -> DocRow {
        let workspace = create_workspace(&self.conn, &format!("Other {n}"), None).expect("workspace");
        let project = get_default_project(&self.conn, &workspace.id).expect("query").expect("inbox");
        let row = self.add_in(&workspace.id, &project.id, n, version);
        assert!(disable_workspace(&self.conn, &workspace.id).expect("disable"));
        row
    }

    /// Like [`World::add`] for a document whose body path is `raw_path`, not the usual one.
    pub(super) fn add_at(&self, n: u32, version: i64, raw_path: &str) -> DocRow {
        let mut row = self.row(&self.project_id, n, version);
        row.raw_path = raw_path.to_string();
        self.write_body(&self.workspace_id, &row, n);
        insert_doc(&self.conn, &row).expect("insert");
        row
    }

    /// A document in the inbox with everything a package can hold: a body, an English
    /// translation (the row says so), a digest at the body path (the row names it) and one
    /// attachment.
    pub(super) fn add_full(&self, n: u32, version: i64) -> DocRow {
        let mut row = self.row(&self.project_id, n, version);
        row.translations = json!(["en"]);
        row.digest_path = Some(row.raw_path.clone());
        let root = self.root();
        self.write_body(&self.workspace_id, &row, n); // also makes the workspace folder
        let translation = expected_lang_common_path(&row.raw_path, "en").expect("translation path");
        write_at(&doc_file_path(&root, "raw", &self.project_id, &translation).expect("path"), &translation_body(n));
        write_at(&doc_file_path(&root, "digest", &self.project_id, &row.raw_path).expect("path"), &digest_body(n));
        let folder = doc_file_path(&root, "raw", &self.project_id, &row.raw_path).expect("path");
        let attached = attached_dir(folder.parent().expect("folder"), &id(n)).expect("attached folder");
        write_at(&attached.join("figure.png"), &attachment_body(n));
        insert_doc(&self.conn, &row).expect("insert");
        row
    }

    /// A document in another project of the Default workspace.
    pub(super) fn add_in_project(&self, project_id: &str, n: u32, version: i64) -> DocRow {
        self.add_in(&self.workspace_id.clone(), project_id, n, version)
    }

    /// Like [`World::add_in_project`] for a body path that is not the usual one.
    pub(super) fn add_in_project_at(&self, project_id: &str, n: u32, version: i64, raw_path: &str) -> DocRow {
        let mut row = self.row(project_id, n, version);
        row.raw_path = raw_path.to_string();
        self.write_body(&self.workspace_id, &row, n);
        insert_doc(&self.conn, &row).expect("insert");
        row
    }

    /// Where a file of the inbox is (or would be) on disk.
    pub(super) fn path(&self, layer: &str, rel: &str) -> PathBuf {
        fs::create_dir_all(self.root()).expect("workspace folder");
        doc_file_path(&self.root(), layer, &self.project_id, rel).expect("path")
    }

    /// The workspace folder of the Default workspace.
    pub(super) fn root(&self) -> PathBuf {
        workspace_dir(&self.workspaces_root, &self.workspace_id).expect("workspace folder")
    }

    /// Every file of the project (both layers, attachments included) by `layer/relative path`.
    /// Two worlds that hold the same documents give the same map whatever their project ids.
    pub(super) fn files_of(&self, project_id: &str) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();
        for layer in ["raw", "digest"] {
            let dir = self.root().join(layer).join(project_id);
            collect(&dir, &dir, layer, &mut files);
        }
        files
    }

    /// [`World::files_of`] the inbox.
    pub(super) fn inbox_files(&self) -> BTreeMap<String, Vec<u8>> {
        self.files_of(&self.project_id)
    }

    /// The row of document `n`, as the database has it now.
    pub(super) fn row_of(&self, n: u32) -> Option<DocRow> {
        crate::services::spark_data::get_doc(&self.conn, &id(n)).expect("query")
    }

    /// The package the world would send for document `n`.
    pub(super) fn package_of(&self, n: u32) -> Vec<u8> {
        let row = self.row_of(n).expect("row");
        pack_doc(&self.root(), &row).expect("pack")
    }

    fn write_body(&self, workspace_id: &str, row: &DocRow, n: u32) {
        let root = workspace_dir(&self.workspaces_root, workspace_id).expect("workspace folder");
        fs::create_dir_all(&root).expect("workspace folder");
        write_at(&doc_file_path(&root, "raw", &row.project_id, &row.raw_path).expect("path"), &body(n));
    }

    fn add_in(&self, workspace_id: &str, project_id: &str, n: u32, version: i64) -> DocRow {
        let row = self.row(project_id, n, version);
        let root = workspace_dir(&self.workspaces_root, workspace_id).expect("workspace folder");
        fs::create_dir_all(&root).expect("workspace folder");
        let path = doc_file_path(&root, "raw", project_id, &row.raw_path).expect("path");
        fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        fs::write(&path, body(n)).expect("body");
        insert_doc(&self.conn, &row).expect("insert");
        row
    }

    fn row(&self, project_id: &str, n: u32, version: i64) -> DocRow {
        let mut row = DocRow::new(&id(n), &format!("notes/20260619{n:04}-topic.md"), project_id);
        row.version = version;
        row
    }

    pub(super) fn remove_row(&self, n: u32) {
        self.conn.execute("DELETE FROM document WHERE doc_id = ?1", [id(n)]).expect("delete");
    }

    pub(super) fn set_version(&self, n: u32, version: i64) {
        self.conn
            .execute("UPDATE document SET version = ?2 WHERE doc_id = ?1", rusqlite::params![id(n), version])
            .expect("update");
    }
}

pub(super) fn write_at(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, bytes).expect("write");
}

fn collect(base: &Path, dir: &Path, layer: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(base, &path, layer, out);
        } else {
            let rel = path.strip_prefix(base).expect("under base").to_string_lossy().replace('\\', "/");
            out.insert(format!("{layer}/{rel}"), fs::read(&path).expect("read"));
        }
    }
}

pub(super) fn translation_body(n: u32) -> Vec<u8> {
    format!("english {n}").into_bytes()
}

pub(super) fn digest_body(n: u32) -> Vec<u8> {
    format!("# digest {n}\n").into_bytes()
}

pub(super) fn attachment_body(n: u32) -> Vec<u8> {
    vec![0x89, b'P', b'N', b'G', 0, n as u8, 255]
}

/// The bytes of the body of document `n`: not text, so a package must carry bytes.
pub(super) fn body(n: u32) -> Vec<u8> {
    let mut bytes = format!("# Doc {n}\r\n").into_bytes();
    bytes.extend_from_slice(&[0xff, 0x00, n as u8]);
    bytes
}

fn upload(doc_id: String, version: i64) -> SyncAction {
    SyncAction::Upload { doc_id, version }
}

#[test]
fn compare_plans_the_cloud_list_against_every_document_of_the_database() {
    let world = World::new();
    world.add(1, 5); // newer here
    world.add(2, 3); // same
    world.add(3, 1); // older here
    world.add(4, 0); // not in the cloud
    let cloud = FakeCloud::new().with(&id(1), 2).with(&id(2), 3).with(&id(3), 4).with(&id(9), 7);

    let actions = compare(&world.conn, &cloud).expect("compare");

    assert_eq!(
        actions,
        [
            upload(id(1), 5),
            SyncAction::Skip { doc_id: id(2) },
            SyncAction::Pull { doc_id: id(3), version: 4 },
            upload(id(4), 0),
            SyncAction::Pull { doc_id: id(9), version: 7 },
        ]
    );
    assert_eq!(cloud.calls(), [Call::List], "comparing sends nothing");
}

#[test]
fn compare_counts_the_documents_of_a_disabled_workspace_too() {
    let world = World::new();
    world.add(1, 1);
    world.add_in_disabled_workspace(2, 6);
    let cloud = FakeCloud::new();

    let actions = compare(&world.conn, &cloud).expect("compare");

    assert_eq!(actions, [upload(id(1), 1), upload(id(2), 6)]);
}

#[test]
fn compare_with_nothing_on_either_side_plans_nothing() {
    let world = World::new();

    assert_eq!(compare(&world.conn, &FakeCloud::new()).expect("compare"), []);
}

#[test]
fn compare_fails_when_the_cloud_cannot_be_listed() {
    let world = World::new();
    world.add(1, 1);
    let cloud = FakeCloud::new().failing_list(CloudSyncError::SessionExpired);

    let err = compare(&world.conn, &cloud).unwrap_err();

    assert_eq!(err, DocSyncError::Cloud(CloudSyncError::SessionExpired));
}

#[test]
fn compare_fails_when_the_database_cannot_be_read() {
    let world = World::new();
    world.conn.execute("DROP TABLE document", []).expect("drop");

    let err = compare(&world.conn, &FakeCloud::new()).unwrap_err();

    assert!(matches!(err, DocSyncError::Database(_)), "{err:?}");
}

#[test]
fn compare_after_an_upload_says_skip() {
    let world = World::new();
    world.add(1, 4);
    let cloud = FakeCloud::new();
    let first = compare(&world.conn, &cloud).expect("first");
    assert_eq!(first, [upload(id(1), 4)]);

    let report = push_uploads(&world.conn, &world.workspaces_root, &cloud, &first, &SyncProgress::new());
    assert_eq!(report.uploaded.len(), 1);

    assert_eq!(compare(&world.conn, &cloud).expect("second"), [SyncAction::Skip { doc_id: id(1) }]);
}
