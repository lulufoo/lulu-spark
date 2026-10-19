//! Shared test fixtures for unit tests (compiled only under `#[cfg(test)]`).

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::config::settings;

mod config_env;
mod notes_fixture;
mod sandbox_layout;

pub(crate) use notes_fixture::{block_note_writes, read_note_entries, seed_note_entries};

pub fn default_workspace() -> crate::services::spark_data::WorkspaceRow {
    let conn = crate::services::spark_data::open().expect("db");
    crate::services::spark_data::get_default_workspace(&conn).expect("default workspace")
}

pub fn knowledge_workspace() -> crate::services::spark_data::WorkspaceRow {
    let conn = crate::services::spark_data::open().expect("db");
    crate::services::spark_data::get_workspace_by_name(&conn, "Knowledge")
        .expect("lookup")
        .expect("Knowledge")
}

pub fn default_workspace_dir() -> PathBuf {
    crate::config::paths::workspaces_root()
        .expect("workspaces root")
        .join(default_workspace().id)
}

pub(crate) use config_env::{install_env_restore_probe, install_test_config_env_construction_probe};
pub use config_env::TestConfigEnv;

pub use sandbox_layout::ProdPathGuardError;
use sandbox_layout::{
    assert_path_not_under_prod_roots, prepare_sandbox_roots, write_fake_prod_config,
    write_sandbox_config,
};

pub(crate) static CONFIG_TEST_SERIAL: Mutex<()> = Mutex::new(());

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
    if depth == 0 {
        settings::register_test_machine_config_path(settings::prod_config_file_path());
    }
    CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(depth + 1));
    let _depth_guard = DepthGuard;
    f();
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

/// Isolated temp HOME + `TestSandbox`/`TestSandboxId` + instance `config.toml`.
pub struct TestSandbox {
    config_env: TestConfigEnv,
    dir: tempfile::TempDir,
    prod_spark_root: PathBuf,
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
        let prod_spark_root = prod.spark_root.clone();
        let prod_cache_dir = prod.cache_dir.clone();

        let sandbox_dir = config_path.parent().expect("sandbox dir").to_path_buf();
        let (wb, cache) = prepare_sandbox_roots(&sandbox_dir);
        write_sandbox_config(&config_path, &wb, &cache, ports);
        let cfg = settings::load().expect("load");
        for path in [cfg.spark_root.as_path(), cfg.cache_dir.as_path()] {
            assert_path_not_under_prod_roots(path, &prod_spark_root, &prod_cache_dir)
            .expect("TestSandbox must not use prod roots");
        }

        Self {
            config_env,
            dir,
            prod_spark_root,
            prod_cache_dir,
        }
    }

    /// Temp HOME of this sandbox (fake prod roots live here as `prod-*`).
    pub fn config_dir(&self) -> &Path {
        self.dir.path()
    }

    /// `<HOME>/.config/lulu-spark-sandbox/<id>`: holds `config.toml` and `cache/`.
    pub fn sandbox_dir(&self) -> PathBuf {
        self.config_file_path()
            .parent()
            .expect("sandbox dir")
            .to_path_buf()
    }

    pub fn config_file_path(&self) -> PathBuf {
        self.config_env.config_file_path()
    }

    pub fn ports(&self) -> (u16, u16) {
        self.config_env.ports()
    }

    pub fn spark_root(&self) -> PathBuf {
        settings::load()
            .expect("load sandbox config")
            .spark_root
    }

    pub fn cache_dir(&self) -> PathBuf {
        settings::load().expect("load sandbox config").cache_dir
    }

    pub fn data_dir(&self) -> PathBuf {
        let dir = settings::runtime_cache_dir()
            .expect("sandbox cache dir")
            .join("data");
        std::fs::create_dir_all(&dir).expect("mkdir data");
        dir
    }

    pub fn prod_spark_root(&self) -> &Path {
        &self.prod_spark_root
    }

    pub fn prod_cache_dir(&self) -> &Path {
        &self.prod_cache_dir
    }

    pub fn assert_not_prod_path(&self, path: &Path) -> Result<(), ProdPathGuardError> {
        assert_path_not_under_prod_roots(path, &self.prod_spark_root, &self.prod_cache_dir)
    }
}

/// Concatenate every `.rs` file under a source directory tree (sorted by path).
pub fn read_rs_dir(dir: impl AsRef<Path>) -> String {
    let mut files = Vec::new();
    collect_rs_files(dir.as_ref(), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|p| std::fs::read_to_string(&p).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Full in-memory vault tree plus store-fail state. LLM-only clear stays on
/// `secrets::test_secrets_clear` so parallel MCP tickets are not wiped.
pub fn reset_vault() {
    crate::config::vault::test_clear_scope();
}

pub fn with_sandbox_notes<F: FnOnce(&Path, &Path)>(f: F) {
    let sandbox = TestSandbox::new();
    let notes = sandbox.data_dir().join("notes");
    std::fs::create_dir_all(&notes).expect("mkdir");
    f(sandbox.config_dir(), notes.as_path());
}

#[path = "unit-tests/test_support.rs"]
mod tests;
