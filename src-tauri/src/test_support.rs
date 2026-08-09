//! Shared test fixtures for unit tests (compiled only under `#[cfg(test)]`).

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::config::settings;

static CONFIG_TEST_SERIAL: Mutex<()> = Mutex::new(());

thread_local! {
    static CONFIG_TEST_SERIAL_DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Serialize tests that mutate process env / sandbox config dirs.
/// Same-thread re-entry is allowed so nested `TestSandbox::new` does not deadlock.
pub fn with_config_test_serial<F: FnOnce()>(f: F) {
    struct DepthGuard;
    impl Drop for DepthGuard {
        fn drop(&mut self) {
            CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        }
    }

    let depth = CONFIG_TEST_SERIAL_DEPTH.with(|d| d.get());
    let _lock: Option<MutexGuard<'_, ()>> = if depth == 0 {
        Some(
            CONFIG_TEST_SERIAL
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        )
    } else {
        None
    };
    CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(depth + 1));
    let _depth_guard = DepthGuard;
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

struct EnvRestore {
    home: Option<String>,
    test_sandbox: Option<String>,
    test_sandbox_id: Option<String>,
}

impl EnvRestore {
    fn capture_and_apply(home: &Path, sandbox_id: &str) -> Self {
        let prev = Self {
            home: std::env::var("HOME").ok(),
            test_sandbox: std::env::var("TestSandbox").ok(),
            test_sandbox_id: std::env::var("TestSandboxId").ok(),
        };
        // SAFETY: test-only env mutation under CONFIG_TEST_SERIAL
        unsafe {
            std::env::set_var("HOME", home);
            std::env::set_var("TestSandbox", "true");
            std::env::set_var("TestSandboxId", sandbox_id);
        }
        prev
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        unsafe {
            match self.home.take() {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match self.test_sandbox.take() {
                Some(v) => std::env::set_var("TestSandbox", v),
                None => std::env::remove_var("TestSandbox"),
            }
            match self.test_sandbox_id.take() {
                Some(v) => std::env::set_var("TestSandboxId", v),
                None => std::env::remove_var("TestSandboxId"),
            }
        }
    }
}

fn unique_sandbox_id(prefix: &str) -> String {
    format!(
        "{prefix}{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn write_fake_prod_config(home: &Path) {
    let prod_dir = home.join(".config").join("lulu-workbench");
    std::fs::create_dir_all(&prod_dir).expect("mkdir prod config");
    let prod_wb = home.join("prod-wb");
    let prod_corpus = home.join("prod-corpus");
    let prod_cache = home.join("prod-cache");
    std::fs::create_dir_all(&prod_wb).expect("mkdir prod wb");
    std::fs::create_dir_all(&prod_corpus).expect("mkdir prod corpus");
    std::fs::create_dir_all(&prod_cache).expect("mkdir prod cache");
    std::fs::write(
        prod_dir.join("config.toml"),
        format!(
            "workbench_knowledge_root = \"{}\"\nknowledge_corpus_root = \"{}\"\ncache_dir = \"{}\"\nmeili_url = \"http://127.0.0.1:7700\"\nhttp_port = 8765\nmcp_port = 9876\n",
            prod_wb.display(),
            prod_corpus.display(),
            prod_cache.display()
        ),
    )
    .expect("write fake prod config");
}

/// Isolated temp HOME + `TestSandbox`/`TestSandboxId` + instance `config.toml`.
pub struct TestSandbox {
    dir: tempfile::TempDir,
    _env: EnvRestore,
    prod_workbench_knowledge_root: PathBuf,
    prod_knowledge_corpus_root: PathBuf,
    prod_cache_dir: PathBuf,
    _config_lock: Option<MutexGuard<'static, ()>>,
}

impl TestSandbox {
    pub fn new() -> Self {
        let depth = CONFIG_TEST_SERIAL_DEPTH.with(|d| d.get());
        let config_lock = if depth == 0 {
            Some(
                CONFIG_TEST_SERIAL
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()),
            )
        } else {
            None
        };
        CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(depth + 1));

        let dir = tempfile::tempdir().expect("tmp");
        let id = unique_sandbox_id("t");
        let env = EnvRestore::capture_and_apply(dir.path(), &id);
        write_fake_prod_config(dir.path());

        let prod = settings::load_prod_settings();
        let prod_workbench_knowledge_root = prod.workbench_knowledge_root.clone();
        let prod_knowledge_corpus_root = prod.knowledge_corpus_root.clone();
        let prod_cache_dir = prod.cache_dir.clone();

        let (wb, corpus, cache) = prepare_sandbox_roots(dir.path());
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
            _env: env,
            prod_workbench_knowledge_root,
            prod_knowledge_corpus_root,
            prod_cache_dir,
            _config_lock: config_lock,
        }
    }

    pub fn config_dir(&self) -> &Path {
        self.dir.path()
    }

    pub fn workbench_knowledge_root(&self) -> PathBuf {
        settings::load()
            .expect("load sandbox config")
            .workbench_knowledge_root
    }

    pub fn knowledge_corpus_root(&self) -> PathBuf {
        settings::load()
            .expect("load sandbox config")
            .knowledge_corpus_root
    }

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

    pub fn assert_not_prod_path(&self, path: &Path) -> Result<(), ProdPathGuardError> {
        assert_path_not_under_prod_roots(
            path,
            &self.prod_workbench_knowledge_root,
            &self.prod_knowledge_corpus_root,
            &self.prod_cache_dir,
        )
    }
}

impl Drop for TestSandbox {
    fn drop(&mut self) {
        drop(self._config_lock.take());
        CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

pub fn with_sandbox<F: FnOnce(&Path)>(f: F) {
    let sandbox = TestSandbox::new();
    f(sandbox.config_dir());
}

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

/// Same isolation as `TestSandbox` (kept for call-site compatibility).
pub fn with_test_config_dir<F: FnOnce(&std::path::Path)>(f: F) {
    let sandbox = TestSandbox::new();
    f(sandbox.config_dir());
}

/// Corpus helper: `f(temp_home, corpus_path)` under `TestSandbox` env.
pub fn with_corpus<F: FnOnce(tempfile::TempDir, PathBuf)>(prepare_ai_subdir: bool, f: F) {
    let depth = CONFIG_TEST_SERIAL_DEPTH.with(|d| d.get());
    let _lock: Option<MutexGuard<'static, ()>> = if depth == 0 {
        Some(
            CONFIG_TEST_SERIAL
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        )
    } else {
        None
    };
    CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(depth + 1));

    let dir = tempfile::tempdir().expect("tmp");
    let id = unique_sandbox_id("c");
    let env = EnvRestore::capture_and_apply(dir.path(), &id);
    write_fake_prod_config(dir.path());
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
    write_sandbox_config(dir.path(), &wb, &corpus, &cache);
    let corpus_path = corpus;
    f(dir, corpus_path);
    drop(env);
    drop(_lock);
    CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
}
