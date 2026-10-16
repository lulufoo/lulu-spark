//! Reading a document's files: `get_file` (a layer's Markdown), `get_digest`, and
//! `export_file` (copy the raw file into a caller-chosen directory).

use std::fs;
use std::path::PathBuf;

use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::file_export::{copy_into_dir_versioned, copy_result};
use crate::services::notes::store::doc_file_path;
use crate::services::spark_data::{DocRow, ProjectRow, WorkspaceRow};

use super::resolve::{
    bad_request, doc_id_field, internal, not_found, open_data, resolve_doc, resolve_workspace,
    text_field, workspace_root,
};

const LAYERS: &[&str] = &["raw", "digest"];

fn layer_rel(doc: &DocRow, layer: &str, lang: Option<&str>) -> Result<String, Value> {
    match (layer, lang) {
        ("raw", Some(lang)) => expected_lang_common_path(&doc.raw_path, lang)
            .ok_or_else(|| bad_request("Invalid path")),
        ("digest", _) => Ok(doc.digest_path.clone().unwrap_or_else(|| doc.raw_path.clone())),
        _ => Ok(doc.raw_path.clone()),
    }
}

/// Where the document's file for `layer` (and translation `lang`) lives on disk.
fn locate(
    workspace: &WorkspaceRow,
    project: &ProjectRow,
    doc: &DocRow,
    layer: &str,
    lang: Option<&str>,
) -> Result<PathBuf, Value> {
    let rel = layer_rel(doc, layer, lang)?;
    if rel.is_empty() || rel.contains("..") {
        return Err(bad_request("Invalid path"));
    }
    doc_file_path(&workspace_root(workspace)?, layer, &project.folder, &rel)
}

fn find(payload: &Value) -> Result<(WorkspaceRow, ProjectRow, DocRow), Value> {
    let id = doc_id_field(payload)?;
    let conn = open_data()?;
    let workspace = resolve_workspace(&conn, text_field(payload, "workspace"))?;
    let (doc, project) = resolve_doc(&conn, &workspace, &id)?;
    Ok((workspace, project, doc))
}

/// `{ workspace?, id, layer?, lang? }` → `{ content }`. `layer` is `raw` (default) or `digest`.
pub fn get_file(payload: &Value) -> Value {
    let layer = text_field(payload, "layer").unwrap_or("raw");
    if !LAYERS.contains(&layer) {
        return bad_request(format!("Invalid layer: {layer}"));
    }
    let (workspace, project, doc) = match find(payload) {
        Ok(found) => found,
        Err(v) => return v,
    };
    let target = match locate(&workspace, &project, &doc, layer, text_field(payload, "lang")) {
        Ok(p) => p,
        Err(v) => return v,
    };
    if !target.is_file() {
        return not_found("File not found");
    }
    match fs::read_to_string(&target) {
        Ok(content) => json!({ "content": content }),
        Err(e) => internal(e.to_string()),
    }
}

/// `{ workspace?, id }` → `{ content }` of the digest layer.
pub fn get_digest(payload: &Value) -> Value {
    let mut request = payload.clone();
    request["layer"] = json!("digest");
    get_file(&request)
}

/// `{ workspace?, id, dest_dir }` → `{ id, ok, path }`: a versioned copy of the raw file.
pub fn export_file(payload: &Value) -> Value {
    let Some(dest_dir) = text_field(payload, "dest_dir") else {
        return bad_request("Missing dest_dir");
    };
    let (workspace, project, doc) = match find(payload) {
        Ok(found) => found,
        Err(v) => return v,
    };
    let source = match locate(&workspace, &project, &doc, "raw", None) {
        Ok(p) if p.is_file() => p,
        Ok(_) => return not_found("File not found"),
        Err(v) => return v,
    };
    copy_result(&doc.doc_id, copy_into_dir_versioned(&source, dest_dir, &doc.doc_id, "md"))
}
