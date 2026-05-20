//! Corpus annotation file paths (single source for read + write).

use std::path::{Path, PathBuf};

fn annotation_rel_path(common_path: &str) -> String {
    if common_path.ends_with(".md") {
        format!("{}.json", &common_path[..common_path.len() - 3])
    } else {
        format!("{common_path}.json")
    }
}

/// Resolve `annotations/{rel}.json` under `corpus`, rejecting traversal.
pub fn annotation_json_path(corpus: &Path, common_path: &str) -> Option<PathBuf> {
    let cp = common_path.trim();
    if cp.is_empty() || cp.contains("..") || cp.starts_with('/') {
        return None;
    }
    let rel = annotation_rel_path(cp);
    if rel.contains("..") {
        return None;
    }
    let base = corpus.join("annotations");
    let target = base.join(&rel);
    let corpus_canon = corpus.canonicalize().ok()?;
    let base_canon = base.canonicalize().ok()?;
    let target_canon = target.canonicalize().ok().or_else(|| {
        let parent = target.parent()?;
        let parent_canon = parent.canonicalize().ok()?;
        Some(parent_canon.join(target.file_name()?))
    })?;
    let prefix = format!("{}{}", corpus_canon.to_string_lossy(), std::path::MAIN_SEPARATOR);
    if !target_canon.to_string_lossy().starts_with(&prefix) {
        return None;
    }
    let ann_prefix = format!("{}{}", base_canon.to_string_lossy(), std::path::MAIN_SEPARATOR);
    if !target_canon.to_string_lossy().starts_with(&ann_prefix) {
        return None;
    }
    Some(target_canon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn md_common_path_maps_to_annotations_json() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(corpus.join("annotations/ai")).expect("mkdir");
        let p = annotation_json_path(&corpus, "ai/note.md").expect("path");
        assert!(p.ends_with("annotations/ai/note.json"));
    }

    #[test]
    fn rejects_traversal() {
        let dir = tempfile::tempdir().expect("tmp");
        let corpus = dir.path().join("corpus");
        fs::create_dir_all(&corpus).expect("mkdir");
        assert!(annotation_json_path(&corpus, "../x.md").is_none());
    }
}
