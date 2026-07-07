//! Shared test fixtures for unit tests (compiled only under `#[cfg(test)]`).

use std::path::{Path, PathBuf};

use crate::config::settings;

fn prepare_sandbox_roots(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let wb = dir.join("workbench-knowledge");
    let corpus = dir.join("corpus");
    let cache = dir.join("cache");
    std::fs::create_dir_all(&wb).expect("mkdir wb");
    std::fs::create_dir_all(&corpus).expect("mkdir corpus");
    std::fs::create_dir_all(&cache).expect("mkdir cache");
    (wb, corpus, cache)
}

fn write_sandbox_config(dir: &Path, wb: &Path, corpus: &Path, cache: &Path) {
    settings::write_test_config_with_cache(dir, wb, Some(corpus), Some(cache));
}

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

fn path_under_prod_root(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

/// Error returned when a test write targets a prod root path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProdPathGuardError {
    pub path: PathBuf,
    pub prod_root: PathBuf,
}

impl std::fmt::Display for ProdPathGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "test path {} must not touch prod root {}",
            self.path.display(),
            self.prod_root.display()
        )
    }
}

impl std::error::Error for ProdPathGuardError {}

fn assert_path_not_under_prod_roots(
    path: &Path,
    prod_wb: &Path,
    prod_corpus: &Path,
    prod_cache: &Path,
) -> Result<(), ProdPathGuardError> {
    for root in [prod_wb, prod_corpus, prod_cache] {
        if path_under_prod_root(path, root) {
            return Err(ProdPathGuardError {
                path: path.to_path_buf(),
                prod_root: root.to_path_buf(),
            });
        }
    }
    Ok(())
}

/// Isolated temp config + sandbox roots (`workbench_knowledge_root`, `knowledge_corpus_root`, `cache_dir`).
pub struct TestSandbox {
    dir: tempfile::TempDir,
    _guard: ConfigDirGuard,
    prod_workbench_knowledge_root: PathBuf,
    prod_knowledge_corpus_root: PathBuf,
    prod_cache_dir: PathBuf,
}

impl TestSandbox {
    pub fn new() -> Self {
        let defaults = settings::AppSettings::default();
        let prod_workbench_knowledge_root = defaults.workbench_knowledge_root.clone();
        let prod_knowledge_corpus_root = defaults.knowledge_corpus_root.clone();
        let prod_cache_dir = defaults.cache_dir.clone();

        let dir = tempfile::tempdir().expect("tmp");
        let (wb, corpus, cache) = prepare_sandbox_roots(dir.path());
        let guard = ConfigDirGuard::install(dir.path().to_path_buf());
        write_sandbox_config(dir.path(), &wb, &corpus, &cache);

        let cfg = settings::load().expect("load");
        for path in [
            cfg.workbench_knowledge_root.as_path(),
            cfg.knowledge_corpus_root.as_path(),
            cfg.cache_dir.as_path(),
        ] {
            assert_path_not_under_prod_roots(
                path,
                &prod_workbench_knowledge_root,
                &prod_knowledge_corpus_root,
                &prod_cache_dir,
            )
            .expect("TestSandbox must not use prod roots");
        }

        Self {
            dir,
            _guard: guard,
            prod_workbench_knowledge_root,
            prod_knowledge_corpus_root,
            prod_cache_dir,
        }
    }

    pub fn config_dir(&self) -> &Path {
        self.dir.path()
    }

    /// Sandbox `workbench_knowledge_root` from loaded test config.
    pub fn workbench_knowledge_root(&self) -> PathBuf {
        settings::load()
            .expect("load sandbox config")
            .workbench_knowledge_root
    }

    pub fn prod_workbench_knowledge_root(&self) -> &Path {
        &self.prod_workbench_knowledge_root
    }

    pub fn prod_knowledge_corpus_root(&self) -> &Path {
        &self.prod_knowledge_corpus_root
    }

    pub fn prod_cache_dir(&self) -> &Path {
        &self.prod_cache_dir
    }

    /// Reject writes that would touch prod `workbench_knowledge_root`, `knowledge_corpus_root`, or `cache_dir`.
    pub fn assert_not_prod_path(&self, path: &Path) -> Result<(), ProdPathGuardError> {
        assert_path_not_under_prod_roots(
            path,
            &self.prod_workbench_knowledge_root,
            &self.prod_knowledge_corpus_root,
            &self.prod_cache_dir,
        )
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
    write_sandbox_config(dir.path(), &wb, &corpus, &cache);
    f(dir, corpus);
}
