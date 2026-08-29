//! Notes annotation file paths (single source for read + write).

use std::path::{Component, Path, PathBuf};

fn annotation_rel_path(common_path: &str) -> String {
    if common_path.ends_with(".md") {
        format!("{}.json", &common_path[..common_path.len() - 3])
    } else {
        format!("{common_path}.json")
    }
}

fn path_has_only_normal_components(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_)))
}

/// Resolve `annotations/{rel}.json` under notes root, rejecting traversal.
/// Does not require `annotations/{topic}/` to exist yet (writes create parents).
pub fn annotation_json_path(notes: &Path, common_path: &str) -> Option<PathBuf> {
    let cp = common_path.trim();
    if cp.is_empty() || cp.contains("..") || cp.starts_with('/') {
        return None;
    }
    if !path_has_only_normal_components(Path::new(cp)) {
        return None;
    }
    let rel = annotation_rel_path(cp);
    if rel.contains("..") || !path_has_only_normal_components(Path::new(&rel)) {
        return None;
    }
    let notes_canon = notes.canonicalize().ok()?;
    let ann_root = notes_canon.join("annotations");
    let target = ann_root.join(&rel);
    if !target.starts_with(&ann_root) {
        return None;
    }
    Some(target)
}

#[cfg(test)]
#[path = "../unit-tests/repositories/annotation_paths.rs"]
mod tests;
