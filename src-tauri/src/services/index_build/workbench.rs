//! Workbench Meilisearch index (`build_workbench_index.py`).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::config::meili_env::notes_root_path;
use crate::integrations::search::{MeiliAdminError, MeiliBackend};

use super::common::{
    build_workbench_document, should_skip_md, upsert_batches, BATCH_SIZE, WORKBENCH_LAYERS,
};

fn walk_md_files(dir: &Path, base: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_md_files(&path, base, out);
        } else if path.is_file() {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name.ends_with(".md") && !should_skip_md(name) {
                if let Ok(rel) = path.strip_prefix(base) {
                    out.push((path.to_path_buf(), rel.to_string_lossy().replace('\\', "/")));
                }
            }
        }
    }
}

pub fn collect_documents(notes_root: &Path) -> Vec<Value> {
    let mut docs = Vec::new();
    for layer in WORKBENCH_LAYERS {
        let layer_dir = notes_root.join(layer);
        if !layer_dir.is_dir() {
            continue;
        }
        let mut files = Vec::new();
        walk_md_files(&layer_dir, &layer_dir, &mut files);
        files.sort_by(|a, b| a.1.cmp(&b.1));
        for (path, common_path) in files {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Ok(body) = fs::read_to_string(&path) else {
                continue;
            };
            docs.push(build_workbench_document(layer, &common_path, &body, name));
        }
    }
    docs.sort_by(|a, b| {
        let la = a["layer"].as_str().unwrap_or("");
        let lb = b["layer"].as_str().unwrap_or("");
        let ca = a["common_path"].as_str().unwrap_or("");
        let cb = b["common_path"].as_str().unwrap_or("");
        (la, ca).cmp(&(lb, cb))
    });
    docs
}

pub fn full_rebuild(repo_root: &Path) -> Result<String, String> {
    let meili = MeiliBackend::new(repo_root);
    meili
        .require_health()
        .map_err(MeiliAdminError::into_message)?;

    let notes = notes_root_path(repo_root);
    meili.wipe_index("workbench").map_err(MeiliAdminError::into_message)?;
    meili
        .ensure_index("workbench", "id")
        .map_err(MeiliAdminError::into_message)?;
    meili
        .put_settings(
            "workbench",
            "searchable-attributes",
            &serde_json::json!(["title", "body", "topic"]),
        )
        .map_err(MeiliAdminError::into_message)?;
    meili
        .put_settings(
            "workbench",
            "filterable-attributes",
            &serde_json::json!(["layer", "common_path"]),
        )
        .map_err(MeiliAdminError::into_message)?;

    let docs = collect_documents(&notes);
    if docs.is_empty() {
        return Ok("No documents found.".to_string());
    }

    let batches: Vec<Vec<Value>> = docs
        .chunks(BATCH_SIZE)
        .map(|c| c.to_vec())
        .collect();
    let total = upsert_batches(&batches, |batch| {
        meili
            .post_documents("workbench", batch)
            .map_err(MeiliAdminError::into_message)
    })?;

    Ok(format!("Total indexed: {total} documents"))
}

#[cfg(test)]
#[path = "../../unit-tests/services/index_build/workbench.rs"]
mod tests;
