//! Running the pull actions of a plan: read the package, put its files on disk, point the row at
//! them and give it the cloud's version.

use std::fs;
use std::path::Path;

use rusqlite::Connection;
use serde_json::{json, Value};

use super::error::DocSyncError;
use super::landing::plan_landing;
use super::plan::SyncAction;
use super::progress::SyncProgress;
use crate::services::cloud_sync::CloudDocs;
use crate::services::doc_zip::{peek_package, unpack_doc_to, Unpacked};
use crate::services::docs::attached::remove_attached_dir;
use crate::services::docs::{doc_file_path, workspace_dir};
use crate::services::id::random_alnum6;
use crate::services::spark_data::{
    apply_pulled_package, find_doc, get_default_project, get_default_workspace, insert_doc, list_docs,
    translation_common_paths, DocRow, FoundDoc,
};

/// What became of one pull.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullOutcome {
    /// The package of `bytes` bytes is on disk and the row has `version`. `created` is true when
    /// the document was not here and got a new row in the Default workspace's inbox.
    Pulled { version: i64, bytes: usize, created: bool, workspace_id: String },
    /// This machine has the document at the cloud's version or higher by now (it was edited, or
    /// pulled, while the package was coming down): nothing was written.
    Kept,
}

/// Brings one document down at `version`, the version the cloud listed. The local row is read
/// afresh once the package is here, so a document that caught up meanwhile is left alone.
///
/// - The document is here, behind: its files are replaced by the package's (what the package
///   lacks goes), the row keeps every column but the paths, `translations` and `version`.
/// - The document is not here: a row is made in the inbox of the default workspace with default
///   columns, `translations` from the package. A name in use there is replaced by one with a
///   suffix.
///
/// The version is stored as the cloud gave it, never plus one. `workspaces_root` is the folder
/// that holds the workspace folders.
pub fn pull_doc(
    conn: &Connection,
    workspaces_root: &Path,
    cloud: &dyn CloudDocs,
    doc_id: &str,
    version: i64,
) -> Result<PullOutcome, DocSyncError> {
    pull_with(conn, workspaces_root, cloud, doc_id, version, random_alnum6)
}

fn pull_with(
    conn: &Connection,
    workspaces_root: &Path,
    cloud: &dyn CloudDocs,
    doc_id: &str,
    version: i64,
    next_suffix: impl FnMut() -> String,
) -> Result<PullOutcome, DocSyncError> {
    let package = cloud.read(doc_id)?;
    let unpack_error = |message: String| DocSyncError::Unpack { doc_id: doc_id.to_string(), message };

    let found = find_doc(conn, doc_id).map_err(DocSyncError::Database)?;
    if found.as_ref().is_some_and(|found| found.doc.version >= version) {
        return Ok(PullOutcome::Kept);
    }
    let (workspace, project_id, existing) = match found {
        Some(FoundDoc { workspace, doc }) => (workspace, doc.project_id.clone(), Some(doc)),
        None => {
            let workspace = get_default_workspace(conn).map_err(DocSyncError::Database)?;
            let inbox = get_default_project(conn, &workspace.id)
                .map_err(DocSyncError::Database)?
                .ok_or_else(|| DocSyncError::Database("the default workspace has no inbox".to_string()))?;
            (workspace, inbox.id, None)
        }
    };
    let root = workspace_dir(workspaces_root, &workspace.id)
        .map_err(|e| unpack_error(e["error"].as_str().unwrap_or("invalid workspace").to_string()))?;
    fs::create_dir_all(&root).map_err(|e| unpack_error(format!("{}: {e}", root.display())))?;

    let peeked = peek_package(&package, doc_id).map_err(unpack_error)?;
    let others: Vec<DocRow> = list_docs(conn, &workspace.id)
        .map_err(DocSyncError::Database)?
        .into_iter()
        .filter(|doc| doc.project_id == project_id)
        .collect();
    let landing = plan_landing(&root, &project_id, doc_id, &peeked, existing.as_ref(), &others, next_suffix)
        .map_err(unpack_error)?;
    let unpacked = unpack_doc_to(&root, &project_id, doc_id, &package, existing.as_ref(), &landing)
        .map_err(unpack_error)?;
    let translations = Value::Array(unpacked.translations.iter().map(|lang| json!(lang)).collect());

    // `(discard the files, why)`. A row that exists keeps pointing at the files it had, and those
    // are the package's by now, so a failed update leaves the document one pull behind: the next
    // run pulls it again. A document with no row would leave files nobody owns.
    let failure = match &existing {
        Some(_) => match apply_pulled_package(
            conn,
            doc_id,
            &unpacked.raw_path,
            unpacked.digest_path.as_deref(),
            &translations,
            version,
        ) {
            Ok(true) => None,
            Ok(false) => Some((true, "the document was deleted during the pull".to_string())),
            Err(message) => Some((false, message)),
        },
        None => {
            let mut row = DocRow::new(doc_id, &unpacked.raw_path, &project_id);
            row.digest_path = unpacked.digest_path.clone();
            row.translations = translations;
            row.version = version;
            insert_doc(conn, &row).err().map(|message| (true, message))
        }
    };
    if let Some((discard, message)) = failure {
        if discard {
            discard_files(&root, &project_id, doc_id, &unpacked);
        }
        return Err(DocSyncError::Database(message));
    }
    Ok(PullOutcome::Pulled { version, bytes: package.len(), created: existing.is_none(), workspace_id: workspace.id })
}

/// Remove the files a pull put on disk, as far as that works: a file already gone is no error.
fn discard_files(root: &Path, project_id: &str, doc_id: &str, unpacked: &Unpacked) {
    let mut row = DocRow::new(doc_id, &unpacked.raw_path, project_id);
    row.translations = Value::Array(unpacked.translations.iter().map(|lang| json!(lang)).collect());
    let mut files = vec![("raw", unpacked.raw_path.clone())];
    files.extend(translation_common_paths(&row).unwrap_or_default().into_iter().map(|(_, rel)| ("raw", rel)));
    files.extend(unpacked.digest_path.iter().map(|rel| ("digest", rel.clone())));
    for (layer, rel) in files {
        if let Ok(path) = doc_file_path(root, layer, project_id, &rel) {
            let _ = fs::remove_file(path);
        }
    }
    if let Some(folder) = doc_file_path(root, "raw", project_id, &unpacked.raw_path)
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        let _ = remove_attached_dir(&folder, doc_id);
    }
}

/// One document that came down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulledDoc {
    pub doc_id: String,
    pub version: i64,
    pub bytes: usize,
    /// True when the document was new here.
    pub created: bool,
    /// The workspace the document is in: its search index needs refreshing.
    pub workspace_id: String,
}

/// The result of [`pull_downloads`]: every pull action is in exactly one of these places.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullReport {
    pub pulled: Vec<PulledDoc>,
    /// Documents this machine had caught up with while the run was going: nothing was written.
    pub kept: Vec<String>,
    /// Documents that could not be pulled for a reason of their own (a package that is no
    /// package, an object another machine deleted meanwhile, a name that cannot be written). The
    /// run went on without them and their local files are as they were.
    pub failed: Vec<(String, DocSyncError)>,
    /// The error that stopped the run (see [`DocSyncError::stops_the_run`]), with the document it
    /// happened on.
    pub stopped: Option<(String, DocSyncError)>,
    /// Pull actions after the one that stopped the run.
    pub not_tried: Vec<String>,
}

/// Runs the `Pull` actions of `actions` in order; other actions are not its business. As with
/// uploads, a failure of one document is recorded and the run goes on, and an error that would
/// hit every document stops it.
///
/// `progress` is told when each document begins and how it ended; a document it does not know is
/// of no interest to it.
pub fn pull_downloads(
    conn: &Connection,
    workspaces_root: &Path,
    cloud: &dyn CloudDocs,
    actions: &[SyncAction],
    progress: &SyncProgress,
) -> PullReport {
    let mut report = PullReport::default();
    for action in actions {
        let SyncAction::Pull { doc_id, version } = action else { continue };
        if report.stopped.is_some() {
            report.not_tried.push(doc_id.clone());
            continue;
        }
        progress.start(doc_id);
        match pull_doc(conn, workspaces_root, cloud, doc_id, *version) {
            Ok(PullOutcome::Pulled { version, bytes, created, workspace_id }) => {
                progress.landed(conn, doc_id);
                report.pulled.push(PulledDoc { doc_id: doc_id.clone(), version, bytes, created, workspace_id })
            }
            Ok(PullOutcome::Kept) => {
                progress.forget(doc_id);
                report.kept.push(doc_id.clone())
            }
            Err(e) if e.stops_the_run() => {
                progress.stopped_on(doc_id, &e);
                report.stopped = Some((doc_id.clone(), e))
            }
            Err(e) => {
                progress.fail(doc_id, &e);
                report.failed.push((doc_id.clone(), e))
            }
        }
    }
    report
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_sync_pull.rs"]
mod tests;
