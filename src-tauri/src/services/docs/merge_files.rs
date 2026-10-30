//! Which relative paths a target project already uses, on disk and in its rows.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::services::spark_data::{translation_common_paths, DocRow};

use super::resolve::internal;
use super::store::doc_file_path;

/// `{workspace root}/{layer}/{project id}`, whether or not it exists.
pub(super) fn get_project_dir(root: &Path, layer: &str, project_id: &str) -> Result<PathBuf, Value> {
    let probe = doc_file_path(root, layer, project_id, "_")?;
    Ok(probe.parent().map(Path::to_path_buf).unwrap_or(probe))
}

/// Every file under `dir`, as a `/`-separated path relative to `dir`.
fn list_rel_files(dir: &Path) -> HashSet<String> {
    fn walk(base: &Path, dir: &Path, out: &mut HashSet<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else if let Ok(rel) = path.strip_prefix(base) {
                let parts: Vec<_> = rel.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                out.insert(parts.join("/"));
            }
        }
    }
    let mut out = HashSet::new();
    walk(dir, dir, &mut out);
    out
}

/// The relative paths the target project already uses: `(raw layer, digest layer)`. Files on
/// disk count, and so do the paths its rows name, whether or not the file is there.
pub(super) fn collect_taken_paths(
    root: &Path,
    target_id: &str,
    target_docs: &[DocRow],
) -> Result<(HashSet<String>, HashSet<String>), Value> {
    let mut raw = list_rel_files(&get_project_dir(root, "raw", target_id)?);
    let mut digest = list_rel_files(&get_project_dir(root, "digest", target_id)?);
    for doc in target_docs {
        raw.insert(doc.raw_path.clone());
        raw.extend(translation_common_paths(doc).map_err(internal)?.into_iter().map(|(_, p)| p));
        digest.extend(doc.digest_path.clone());
    }
    Ok((raw, digest))
}
