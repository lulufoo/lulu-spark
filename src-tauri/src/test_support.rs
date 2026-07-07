//! Shared test fixtures for unit tests (compiled only under `#[cfg(test)]`).

use std::path::{Path, PathBuf};

use crate::config::settings;

/// RAII guard restoring the previous test config dir on drop (supports nested sandboxes).
struct ConfigDirGuard {
    prev: Option<PathBuf>,
}

impl ConfigDirGuard {
    fn install(dir: PathBuf) -> Self {
        let prev = settings::test_config_dir_snapshot();
        settings::set_test_config_dir(Some(dir));
        Self { prev }
    }
}

impl Drop for ConfigDirGuard {
    fn drop(&mut self) {
        settings::set_test_config_dir(self.prev.take());
    }
}

/// Isolated temp config + sandbox roots (`workbench_knowledge_root`, `knowledge_corpus_root`, `cache_dir`).
pub struct TestSandbox {
    dir: tempfile::TempDir,
    _guard: ConfigDirGuard,
}

impl TestSandbox {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tmp");
        let wb = dir.path().join("workbench-knowledge");
        let corpus = dir.path().join("corpus");
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&wb).expect("mkdir wb");
        std::fs::create_dir_all(&corpus).expect("mkdir corpus");
        std::fs::create_dir_all(&cache).expect("mkdir cache");
        let guard = ConfigDirGuard::install(dir.path().to_path_buf());
        settings::write_test_config_with_cache(
            dir.path(),
            &wb,
            Some(&corpus),
            Some(&cache),
        );
        Self { dir, _guard: guard }
    }

    pub fn config_dir(&self) -> &Path {
        self.dir.path()
    }
}

/// Minimal test config helper: temp dir → set config dir → run `f` → clear.
pub fn with_test_config_dir<F: FnOnce(&std::path::Path)>(f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = ConfigDirGuard::install(dir.path().to_path_buf());
    f(dir.path());
}

/// Corpus helper: create corpus tree, write sandbox three-root config, run `f`, restore config dir.
pub fn with_corpus<F: FnOnce(tempfile::TempDir, PathBuf)>(prepare_ai_subdir: bool, f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    if prepare_ai_subdir {
        std::fs::create_dir_all(corpus.join("annotations/ai")).expect("mkdir");
    } else {
        std::fs::create_dir_all(corpus.join("annotations")).expect("mkdir");
    }
    let wb = dir.path().join("workbench-knowledge");
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(&wb).expect("mkdir wb");
    std::fs::create_dir_all(&cache).expect("mkdir cache");
    let _guard = ConfigDirGuard::install(dir.path().to_path_buf());
    settings::write_test_config_with_cache(dir.path(), &wb, Some(&corpus), Some(&cache));
    f(dir, corpus);
}
