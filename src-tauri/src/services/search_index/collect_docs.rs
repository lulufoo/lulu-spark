//! Chunks of the documents in the per-workspace tables (`doc_<workspace id>`).
//!
//! Files live at `{workspaces_root}/{workspace id}/{layer}/{project folder}/{rel}`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::config::paths;
use crate::services::docs::doc_file_path;
use crate::services::spark_data::{
    self, get_doc, get_project, get_workspace, list_docs, list_projects, list_workspaces, DocRow,
    WorkspaceRow,
};

use super::collect::{chunks_from_file, SourceChunk, DOCS_CATEGORY};

fn workspace_root(workspace_id: &str) -> Result<PathBuf, String> {
    let root = paths::workspaces_root().map_err(|e| format!("{e:?}"))?;
    Ok(root.join(workspace_id))
}

/// Chunks of one document: its raw file, plus its digest file when the row says there is one.
/// A file that is missing or unreadable contributes no chunks.
fn doc_chunks(root: &Path, workspace_id: &str, doc: &DocRow, folder: &str) -> Vec<SourceChunk> {
    let created_at = doc.created_at.as_deref().unwrap_or("");
    let mut layers = vec![("raw", doc.raw_path.as_str())];
    if let Some(digest) = doc.digest_path.as_deref().filter(|p| !p.is_empty()) {
        layers.push(("digest", digest));
    }
    layers
        .into_iter()
        .filter(|(_, rel)| !rel.is_empty() && !rel.contains(".."))
        .flat_map(|(layer, rel)| {
            let Ok(abs) = doc_file_path(root, layer, folder, rel) else {
                return Vec::new();
            };
            chunks_from_file(DOCS_CATEGORY, layer, &abs, &doc.doc_id, created_at)
                .into_iter()
                .map(|chunk| chunk.in_workspace(workspace_id))
                .collect()
        })
        .collect()
}

fn workspace_chunks(conn: &Connection, workspace: &WorkspaceRow) -> Result<Vec<SourceChunk>, String> {
    let root = workspace_root(&workspace.id)?;
    let folders: HashMap<String, String> = list_projects(conn, &workspace.id)?
        .into_iter()
        .map(|p| (p.id, p.folder))
        .collect();
    let mut chunks = Vec::new();
    for doc in list_docs(conn, &workspace.doc_table)? {
        if let Some(folder) = folders.get(&doc.project_id) {
            chunks.extend(doc_chunks(&root, &workspace.id, &doc, folder));
        }
    }
    Ok(chunks)
}

/// Chunks of every document of every workspace.
pub fn collect_workspace_docs() -> Result<Vec<SourceChunk>, String> {
    let conn = spark_data::open()?;
    let mut chunks = Vec::new();
    for workspace in list_workspaces(&conn)? {
        chunks.extend(workspace_chunks(&conn, &workspace)?);
    }
    Ok(chunks)
}

/// Chunks of document `doc_id` of `workspace_id`; empty when the document (or the
/// workspace) is gone, which is how a deleted document drops out of the index.
pub fn workspace_doc_chunks(workspace_id: &str, doc_id: &str) -> Result<Vec<SourceChunk>, String> {
    let conn = spark_data::open()?;
    let Some(workspace) = get_workspace(&conn, workspace_id)? else {
        return Ok(Vec::new());
    };
    // A disabled workspace or project drops out of the index, like a deleted document.
    if workspace.disabled_at.is_some() {
        return Ok(Vec::new());
    }
    let Some(doc) = get_doc(&conn, &workspace.doc_table, doc_id)? else {
        return Ok(Vec::new());
    };
    let Some(project) = get_project(&conn, &doc.project_id)?.filter(|p| p.disabled_at.is_none()) else {
        return Ok(Vec::new());
    };
    Ok(doc_chunks(&workspace_root(workspace_id)?, workspace_id, &doc, &project.folder))
}
