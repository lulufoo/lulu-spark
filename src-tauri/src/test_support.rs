//! Shared test fixtures for unit tests (compiled only under `#[cfg(test)]`).

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::config::settings;

pub(crate) static CONFIG_TEST_SERIAL: Mutex<()> = Mutex::new(());

thread_local! {
    static CONFIG_TEST_SERIAL_DEPTH: Cell<u32> = const { Cell::new(0) };
    static ENV_RESTORE_PROBE: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
    static TEST_CONFIG_ENV_CONSTRUCTION_PROBE: RefCell<Option<Box<dyn Fn()>>> =
        const { RefCell::new(None) };
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
    if depth == 0 {
        settings::register_test_machine_config_path(settings::prod_config_file_path());
    }
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

fn write_sandbox_config(
    config_path: &Path,
    wb: &Path,
    corpus: &Path,
    cache: &Path,
    ports: (u16, u16),
) {
    settings::write_test_config_with_cache(
        config_path,
        wb,
        Some(corpus),
        Some(cache),
        "http://127.0.0.1:17700",
        ports.0,
        ports.1,
    )
    .expect("write sandbox config");
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

pub(crate) struct EnvRestoreProbeGuard {
    previous: Option<Box<dyn Fn()>>,
}

impl Drop for EnvRestoreProbeGuard {
    fn drop(&mut self) {
        ENV_RESTORE_PROBE.with(|probe| {
            *probe.borrow_mut() = self.previous.take();
        });
    }
}

pub(crate) fn install_env_restore_probe<F>(probe: F) -> EnvRestoreProbeGuard
where
    F: Fn() + 'static,
{
    let previous =
        ENV_RESTORE_PROBE.with(|current| current.borrow_mut().replace(Box::new(probe)));
    EnvRestoreProbeGuard { previous }
}

pub(crate) struct TestConfigEnvConstructionProbeGuard {
    previous: Option<Box<dyn Fn()>>,
}

impl Drop for TestConfigEnvConstructionProbeGuard {
    fn drop(&mut self) {
        TEST_CONFIG_ENV_CONSTRUCTION_PROBE.with(|probe| {
            *probe.borrow_mut() = self.previous.take();
        });
    }
}

pub(crate) fn install_test_config_env_construction_probe<F>(
    probe: F,
) -> TestConfigEnvConstructionProbeGuard
where
    F: Fn() + 'static,
{
    let previous = TEST_CONFIG_ENV_CONSTRUCTION_PROBE
        .with(|current| current.borrow_mut().replace(Box::new(probe)));
    TestConfigEnvConstructionProbeGuard { previous }
}

struct EnvRestore {
    home: Option<String>,
    test_sandbox: Option<String>,
    test_sandbox_id: Option<String>,
    restored: bool,
}

impl EnvRestore {
    fn capture() -> Self {
        Self {
            home: std::env::var("HOME").ok(),
            test_sandbox: std::env::var("TestSandbox").ok(),
            test_sandbox_id: std::env::var("TestSandboxId").ok(),
            restored: false,
        }
    }

    fn apply_prod(home: &Path) {
        // SAFETY: test-only env mutation under CONFIG_TEST_SERIAL
        unsafe {
            std::env::set_var("HOME", home);
            std::env::remove_var("TestSandbox");
            std::env::remove_var("TestSandboxId");
        }
    }

    fn apply_sandbox(home: &Path, sandbox_id: &str) {
        // SAFETY: test-only env mutation under CONFIG_TEST_SERIAL
        unsafe {
            std::env::set_var("HOME", home);
            std::env::set_var("TestSandbox", "true");
            std::env::set_var("TestSandboxId", sandbox_id);
        }
    }

    fn restore(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        if self.restored {
            return None;
        }
        let probe_panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ENV_RESTORE_PROBE.with(|probe| {
                if let Some(probe) = probe.borrow().as_ref() {
                    probe();
                }
            });
        }));
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
        self.restored = true;
        probe_panic.err()
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestConfigPlane {
    Prod,
    Sandbox { id: String },
}

pub struct TestConfigEnv {
    home: PathBuf,
    plane: TestConfigPlane,
    env: Option<EnvRestore>,
    lock: Option<MutexGuard<'static, ()>>,
}

impl TestConfigEnv {
    pub fn prod(home: &Path) -> Self {
        Self::new(home, TestConfigPlane::Prod)
    }

    pub fn sandbox(home: &Path, id: &str) -> Self {
        Self::new(home, TestConfigPlane::Sandbox { id: id.to_owned() })
    }

    fn new(home: &Path, plane: TestConfigPlane) -> Self {
        let home = home.to_path_buf();
        let depth = CONFIG_TEST_SERIAL_DEPTH.with(|d| d.get());
        let lock = if depth == 0 {
            Some(
                CONFIG_TEST_SERIAL
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()),
            )
        } else {
            None
        };
        if depth == 0 {
            settings::register_test_machine_config_path(settings::prod_config_file_path());
        }
        let env = EnvRestore::capture();
        match &plane {
            TestConfigPlane::Prod => EnvRestore::apply_prod(&home),
            TestConfigPlane::Sandbox { id } => EnvRestore::apply_sandbox(&home, id),
        }
        let construction_panic =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                TEST_CONFIG_ENV_CONSTRUCTION_PROBE.with(|probe| {
                    if let Some(probe) = probe.borrow().as_ref() {
                        probe();
                    }
                });
            }))
            .err();
        if let Some(payload) = construction_panic {
            drop(env);
            drop(lock);
            std::panic::resume_unwind(payload);
        }
        CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(depth + 1));
        Self {
            home,
            plane,
            env: Some(env),
            lock,
        }
    }

    pub fn config_file_path(&self) -> PathBuf {
        let config_dir = match &self.plane {
            TestConfigPlane::Prod => "lulu-workbench".to_owned(),
            TestConfigPlane::Sandbox { id } => format!("lulu-workbench-sandbox-{id}"),
        };
        self.home
            .join(".config")
            .join(config_dir)
            .join(settings::PROD_CONFIG_FILE_NAME)
    }

    pub fn ports(&self) -> (u16, u16) {
        match self.plane {
            TestConfigPlane::Prod => (
                settings::DEFAULT_PROD_HTTP_PORT,
                settings::DEFAULT_PROD_MCP_PORT,
            ),
            TestConfigPlane::Sandbox { .. } => (
                settings::DEFAULT_SANDBOX_HTTP_PORT,
                settings::DEFAULT_SANDBOX_MCP_PORT,
            ),
        }
    }
}

impl Drop for TestConfigEnv {
    fn drop(&mut self) {
        let probe_panic = self.env.as_mut().and_then(EnvRestore::restore);
        drop(self.env.take());
        CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        drop(self.lock.take());
        if let Some(payload) = probe_panic {
            std::panic::resume_unwind(payload);
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
    let prod_config_path = home
        .join(".config")
        .join("lulu-workbench")
        .join(settings::PROD_CONFIG_FILE_NAME);
    let prod_wb = home.join("prod-wb");
    let prod_corpus = home.join("prod-corpus");
    let prod_cache = home.join("prod-cache");
    std::fs::create_dir_all(&prod_wb).expect("mkdir prod wb");
    std::fs::create_dir_all(&prod_corpus).expect("mkdir prod corpus");
    std::fs::create_dir_all(&prod_cache).expect("mkdir prod cache");
    settings::write_test_config_with_cache(
        &prod_config_path,
        &prod_wb,
        Some(&prod_corpus),
        Some(&prod_cache),
        "http://127.0.0.1:7700",
        settings::DEFAULT_PROD_HTTP_PORT,
        settings::DEFAULT_PROD_MCP_PORT,
    )
    .expect("write fake prod config");
}

/// Isolated temp HOME + `TestSandbox`/`TestSandboxId` + instance `config.toml`.
pub struct TestSandbox {
    config_env: TestConfigEnv,
    dir: tempfile::TempDir,
    prod_workbench_knowledge_root: PathBuf,
    prod_knowledge_corpus_root: PathBuf,
    prod_cache_dir: PathBuf,
}

impl TestSandbox {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tmp");
        let id = unique_sandbox_id("t");
        let config_env = TestConfigEnv::sandbox(dir.path(), &id);
        let config_path = config_env.config_file_path();
        let ports = config_env.ports();
        write_fake_prod_config(dir.path());

        let prod = settings::load_prod_settings();
        let prod_workbench_knowledge_root = prod.workbench_knowledge_root.clone();
        let prod_knowledge_corpus_root = prod.knowledge_corpus_root.clone();
        let prod_cache_dir = prod.cache_dir.clone();

        let (wb, corpus, cache) = prepare_sandbox_roots(dir.path());
        write_sandbox_config(&config_path, &wb, &corpus, &cache, ports);
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
            config_env,
            dir,
            prod_workbench_knowledge_root,
            prod_knowledge_corpus_root,
            prod_cache_dir,
        }
    }

    pub fn config_dir(&self) -> &Path {
        self.dir.path()
    }

    pub fn config_file_path(&self) -> PathBuf {
        self.config_env.config_file_path()
    }

    pub fn ports(&self) -> (u16, u16) {
        self.config_env.ports()
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
    let dir = tempfile::tempdir().expect("tmp");
    let id = unique_sandbox_id("c");
    let config_env = TestConfigEnv::sandbox(dir.path(), &id);
    let config_path = config_env.config_file_path();
    let ports = config_env.ports();
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
    write_sandbox_config(&config_path, &wb, &corpus, &cache, ports);
    let corpus_path = corpus;
    f(dir, corpus_path);
}
