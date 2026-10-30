//! File-only relocate steps shared by `move_doc` and `delete_project`.
//! Plan is pure. Stage copies. Cleanup deletes sources. No database, no index.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::services::archive_parse::expected_lang_common_path;
use super::attached::{copy_attached_dir, remove_attached_dir};
use super::merge_plan::{plan_merge_targets, MergeTarget};
use super::store::doc_file_path;
use crate::services::spark_data::{translation_common_paths, DocRow};

pub(crate) struct RelocateFile {
    pub layer: &'static str,
    pub src_rel: String,
    pub dest_rel: String,
}

pub(crate) struct RelocatePlan {
    pub doc_id: String,
    pub src_project_id: String,
    pub dest_project_id: String,
    pub target: MergeTarget,
    pub files: Vec<RelocateFile>,
}

/// Target paths via [`plan_merge_targets`], then the body-file pairs. Attached is not
/// a pair: stage copies the directory first from the source raw parent to the dest raw parent.
pub(crate) fn plan_relocate(
    doc: &DocRow,
    dest_project_id: &str,
    taken_raw: &HashSet<String>,
    taken_digest: &HashSet<String>,
    next_suffix: impl FnMut() -> String,
) -> Result<RelocatePlan, String> {
    let mut targets = plan_merge_targets(std::slice::from_ref(doc), taken_raw, taken_digest, next_suffix)?;
    let target = targets.pop().ok_or_else(|| "empty relocate plan".to_string())?;
    let mut files = vec![RelocateFile {
        layer: "raw",
        src_rel: doc.raw_path.clone(),
        dest_rel: target.raw_path.clone(),
    }];
    for (lang, src_rel) in translation_common_paths(doc)? {
        let dest_rel = expected_lang_common_path(&target.raw_path, &lang)
            .ok_or_else(|| format!("doc {}: invalid translation path", doc.doc_id))?;
        files.push(RelocateFile {
            layer: "raw",
            src_rel,
            dest_rel,
        });
    }
    let src_digest = doc.digest_path.clone().unwrap_or_else(|| doc.raw_path.clone());
    let dest_digest = target
        .digest_path
        .clone()
        .unwrap_or_else(|| target.raw_path.clone());
    files.push(RelocateFile {
        layer: "digest",
        src_rel: src_digest,
        dest_rel: dest_digest,
    });
    Ok(RelocatePlan {
        doc_id: doc.doc_id.clone(),
        src_project_id: doc.project_id.clone(),
        dest_project_id: dest_project_id.to_string(),
        target,
        files,
    })
}

/// Plan each document in order, reserving the dest paths of earlier documents.
pub(crate) fn plan_relocate_all(
    docs: &[DocRow],
    dest_project_id: &str,
    taken_raw: &HashSet<String>,
    taken_digest: &HashSet<String>,
    mut next_suffix: impl FnMut() -> String,
) -> Result<Vec<RelocatePlan>, String> {
    let mut raw = taken_raw.clone();
    let mut digest = taken_digest.clone();
    let mut plans = Vec::with_capacity(docs.len());
    for doc in docs {
        let plan = plan_relocate(doc, dest_project_id, &raw, &digest, &mut next_suffix)?;
        for file in &plan.files {
            if file.layer == "digest" {
                digest.insert(file.dest_rel.clone());
            } else {
                raw.insert(file.dest_rel.clone());
            }
        }
        plans.push(plan);
    }
    Ok(plans)
}

/// Stage order for tests: attached directory, then each body file.
pub(crate) fn stage_sequence(plan: &RelocatePlan) -> Vec<String> {
    let mut steps = vec!["attached".to_string()];
    for file in &plan.files {
        steps.push(format!("{}:{}", file.layer, file.src_rel));
    }
    steps
}

/// Copy the attached directory, then each body file that exists. Sources stay in place.
pub(crate) fn stage_relocate(
    src_root: &Path,
    dest_root: &Path,
    plan: &RelocatePlan,
) -> Result<(), Value> {
    let raw = plan
        .files
        .iter()
        .find(|f| f.layer == "raw")
        .ok_or_else(|| json!({ "error": "Invalid path", "_status": 400 }))?;
    let src_raw = doc_file_path(src_root, "raw", &plan.src_project_id, &raw.src_rel)?;
    let dest_raw = doc_file_path(dest_root, "raw", &plan.dest_project_id, &raw.dest_rel)?;
    let src_md = src_raw.parent().ok_or_else(|| json!({ "error": "Invalid path", "_status": 400 }))?;
    let dest_md = dest_raw.parent().ok_or_else(|| json!({ "error": "Invalid path", "_status": 400 }))?;
    copy_attached_dir(src_md, dest_md, &plan.doc_id)?;
    for file in &plan.files {
        let src = doc_file_path(src_root, file.layer, &plan.src_project_id, &file.src_rel)?;
        if !src.is_file() {
            continue;
        }
        let dest = doc_file_path(dest_root, file.layer, &plan.dest_project_id, &file.dest_rel)?;
        if dest.exists() {
            return Err(json!({ "error": "File already exists", "_status": 409 }));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
        }
        fs::copy(&src, &dest).map_err(|e| json!({ "error": e.to_string(), "_status": 500 }))?;
    }
    Ok(())
}

/// Delete source body files and the source attached directory. Dest is left as-is.
pub(crate) fn cleanup_relocate(src_root: &Path, plan: &RelocatePlan) -> Result<(), Value> {
    for file in &plan.files {
        let src = doc_file_path(src_root, file.layer, &plan.src_project_id, &file.src_rel)?;
        if src.is_file() {
            let _ = fs::remove_file(&src);
        }
    }
    let raw = plan
        .files
        .iter()
        .find(|f| f.layer == "raw")
        .ok_or_else(|| json!({ "error": "Invalid path", "_status": 400 }))?;
    let src_raw = doc_file_path(src_root, "raw", &plan.src_project_id, &raw.src_rel)?;
    if let Some(md_dir) = src_raw.parent() {
        let _ = remove_attached_dir(md_dir, &plan.doc_id);
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_relocate.rs"]
mod tests;
