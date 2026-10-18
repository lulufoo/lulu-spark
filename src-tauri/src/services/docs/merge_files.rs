//! Disk side of merging a project into another: what the target already holds, which files
//! move, and the move itself with a way back.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::services::archive_parse::expected_lang_common_path;
use crate::services::spark_data::{translation_common_paths, DocRow};

use super::merge_plan::MergeTarget;
use super::resolve::{bad_request, internal};
use super::store::doc_file_path;

/// One file move: `(from, to)`.
pub(super) type FileMove = (PathBuf, PathBuf);

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

/// The files of `doc` that exist under project `from_id`, with where each one goes under
/// `to_id` after `target`. The digest file sits at `digest_path`, or at `raw_path` without one.
pub(super) fn list_doc_moves(
    root: &Path,
    from_id: &str,
    to_id: &str,
    doc: &DocRow,
    target: &MergeTarget,
) -> Result<Vec<FileMove>, Value> {
    let mut moves = Vec::new();
    let mut add = |layer: &str, old_rel: &str, new_rel: &str| -> Result<(), Value> {
        let src = doc_file_path(root, layer, from_id, old_rel)?;
        if src.exists() {
            moves.push((src, doc_file_path(root, layer, to_id, new_rel)?));
        }
        Ok(())
    };
    add("raw", &doc.raw_path, &target.raw_path)?;
    for (lang, old_rel) in translation_common_paths(doc).map_err(internal)? {
        let new_rel = expected_lang_common_path(&target.raw_path, &lang).ok_or_else(|| bad_request("Invalid path"))?;
        add("raw", &old_rel, &new_rel)?;
    }
    let old_digest = doc.digest_path.as_deref().unwrap_or(&doc.raw_path);
    let new_digest = target.digest_path.as_deref().unwrap_or(&target.raw_path);
    add("digest", old_digest, new_digest)?;
    Ok(moves)
}

/// Move every file, never over an existing one. On any failure the files already moved go
/// back and the error is returned; on success the moves done are returned for a later
/// [`undo_moves`].
pub(super) fn run_moves(moves: &[FileMove], stops: &[PathBuf]) -> Result<Vec<FileMove>, String> {
    let mut done: Vec<FileMove> = Vec::new();
    for (src, dst) in moves {
        let step = if dst.exists() {
            Err(format!("{} already exists", dst.display()))
        } else {
            dst.parent()
                .map_or(Ok(()), fs::create_dir_all)
                .and_then(|_| fs::rename(src, dst))
                .map_err(|e| e.to_string())
        };
        if let Err(e) = step {
            undo_moves(&done, stops);
            return Err(format!("Move failed, rolled back: {e}"));
        }
        done.push((src.clone(), dst.clone()));
    }
    Ok(done)
}

/// Put the moved files back in reverse order, then drop the folders the moves created.
/// `stops` are folders that stay even when empty.
pub(super) fn undo_moves(done: &[FileMove], stops: &[PathBuf]) {
    for (src, dst) in done.iter().rev() {
        let _ = fs::rename(dst, src);
    }
    for (_, dst) in done {
        remove_empty_parents(dst, stops);
    }
}

/// Remove the empty folders above `file`, up to but not including the `stops` folder it is in.
fn remove_empty_parents(file: &Path, stops: &[PathBuf]) {
    let Some(stop) = stops.iter().find(|s| file.starts_with(s)) else {
        return;
    };
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d == stop.as_path() || fs::remove_dir(d).is_err() {
            break;
        }
        dir = d.parent();
    }
}
