//! Shared test fixtures for unit tests (compiled only under `#[cfg(test)]`).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::config::settings;

static CONFIG_TEST_SERIAL: Mutex<()> = Mutex::new(());

/// Serialize tests that mutate `dev.config.toml` (TestSandbox).
pub fn with_config_test_serial<F: FnOnce()>(f: F) {
    let _guard = CONFIG_TEST_SERIAL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    f();
}

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

fn path_under_prod_root(path: &Path, root: &Path) -> bool {
    if path == root {
        return true;
    }
    path.starts_with(root.join(""))
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

/// Isolated temp data roots + `dev.config.toml` (never prod `config.toml`).
pub struct TestSandbox {
    dir: tempfile::TempDir,
    prod_workbench_knowledge_root: PathBuf,
    prod_knowledge_corpus_root: PathBuf,
    prod_cache_dir: PathBuf,
}

impl TestSandbox {
    pub fn new() -> Self {
        let prod = settings::load_prod_settings();
        let prod_workbench_knowledge_root = prod.workbench_knowledge_root.clone();
        let prod_knowledge_corpus_root = prod.knowledge_corpus_root.clone();
        let prod_cache_dir = prod.cache_dir.clone();

        let dir = tempfile::tempdir().expect("tmp");
        let (wb, corpus, cache) = prepare_sandbox_roots(dir.path());
        with_config_test_serial(|| {
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
        });

        Self {
            dir,
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

    /// Sandbox `knowledge_corpus_root` from loaded test config.
    pub fn knowledge_corpus_root(&self) -> PathBuf {
        settings::load()
            .expect("load sandbox config")
            .knowledge_corpus_root
    }

    /// Sandbox `cache_dir` from loaded test config.
    pub fn cache_dir(&self) -> PathBuf {
        settings::load().expect("load sandbox config").cache_dir
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

/// Isolated sandbox config dir (preferred over bare `with_test_config_dir`).
pub fn with_sandbox<F: FnOnce(&Path)>(f: F) {
    let sandbox = TestSandbox::new();
    f(sandbox.config_dir());
}

/// Corpus fixture on `TestSandbox`; files live under sandbox `workbench_knowledge_root`
/// (matches service path resolution via `workbench_knowledge_root_path`).
pub fn with_sandbox_corpus<F: FnOnce(&Path, &Path)>(prepare_ai_subdir: bool, f: F) {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    if prepare_ai_subdir {
        std::fs::create_dir_all(wb.join("annotations/ai")).expect("mkdir");
    } else {
        std::fs::create_dir_all(wb.join("annotations")).expect("mkdir");
    }
    f(sandbox.config_dir(), wb.as_path());
}

struct IsolatedConfigDirGuard {
    prev: Option<String>,
}

impl IsolatedConfigDirGuard {
    fn set(dir: &Path) -> Self {
        let prev = std::env::var("LULU_WB_CONFIG_DIR").ok();
        // SAFETY: test-only env mutation
        unsafe { std::env::set_var("LULU_WB_CONFIG_DIR", dir) };
        Self { prev }
    }
}

impl Drop for IsolatedConfigDirGuard {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(v) => unsafe { std::env::set_var("LULU_WB_CONFIG_DIR", v) },
            None => unsafe { std::env::remove_var("LULU_WB_CONFIG_DIR") },
        }
    }
}

/// Temp config dir via `LULU_WB_CONFIG_DIR` (uses `config.toml` inside, not dev.config.toml).
pub fn with_test_config_dir<F: FnOnce(&std::path::Path)>(f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigDirGuard::set(dir.path());
    f(dir.path());
}

/// Corpus helper: create corpus tree, write sandbox three-root config, run `f`.
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
    let _guard = IsolatedConfigDirGuard::set(dir.path());
    write_sandbox_config(dir.path(), &wb, &corpus, &cache);
    f(dir, corpus);
}
