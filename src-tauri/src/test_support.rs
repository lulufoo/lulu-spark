//! Shared test fixtures for unit tests (compiled only under `#[cfg(test)]`).

use std::path::PathBuf;

use crate::config::settings;

/// Minimal test config helper: temp dir → set config dir → run `f` → clear.
pub fn with_test_config_dir<F: FnOnce(&std::path::Path)>(f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    settings::set_test_config_dir(Some(dir.path().to_path_buf()));
    f(dir.path());
    settings::set_test_config_dir(None);
}

/// Corpus helper: create corpus tree, write test config, run `f`, clear config dir.
pub fn with_corpus<F: FnOnce(tempfile::TempDir, PathBuf)>(prepare_ai_subdir: bool, f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    if prepare_ai_subdir {
        std::fs::create_dir_all(corpus.join("annotations/ai")).expect("mkdir");
    } else {
        std::fs::create_dir_all(corpus.join("annotations")).expect("mkdir");
    }
    settings::write_test_config(dir.path(), &corpus, None);
    f(dir, corpus);
    settings::set_test_config_dir(None);
}
