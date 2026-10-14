//! Process env switch for config tests: temp HOME + Prod / Sandbox plane,
//! restored on Drop under the shared config serial lock.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;

use crate::config::settings;

thread_local! {
    static ENV_RESTORE_PROBE: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
    static TEST_CONFIG_ENV_CONSTRUCTION_PROBE: RefCell<Option<Box<dyn Fn()>>> =
        const { RefCell::new(None) };
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

    fn apply_raw(home: &Path, flag: Option<&str>, sandbox_id: Option<&str>) {
        // SAFETY: test-only env mutation under CONFIG_TEST_SERIAL
        unsafe {
            std::env::set_var("HOME", home);
            match flag {
                Some(v) => std::env::set_var("TestSandbox", v),
                None => std::env::remove_var("TestSandbox"),
            }
            match sandbox_id {
                Some(v) => std::env::set_var("TestSandboxId", v),
                None => std::env::remove_var("TestSandboxId"),
            }
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
    /// Arbitrary `TestSandbox` / `TestSandboxId` values (mode-resolution tests).
    Raw { flag: Option<String>, id: Option<String> },
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

    /// Set raw env values, including invalid ones; config paths/ports report the prod plane.
    pub fn raw(home: &Path, flag: Option<&str>, id: Option<&str>) -> Self {
        Self::new(
            home,
            TestConfigPlane::Raw {
                flag: flag.map(str::to_owned),
                id: id.map(str::to_owned),
            },
        )
    }

    fn new(home: &Path, plane: TestConfigPlane) -> Self {
        let home = home.to_path_buf();
        let depth = super::CONFIG_TEST_SERIAL_DEPTH.with(|d| d.get());
        let lock = if depth == 0 {
            Some(
                super::CONFIG_TEST_SERIAL
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
            TestConfigPlane::Raw { flag, id } => {
                EnvRestore::apply_raw(&home, flag.as_deref(), id.as_deref())
            }
        }
        let construction_panic =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                TEST_CONFIG_ENV_CONSTRUCTION_PROBE.with(|probe| {
                    if let Some(probe) = probe.borrow().as_ref() {
                        probe();
                    }
                });
                settings::sync_runtime_plane_from_env();
            }))
            .err();
        if let Some(payload) = construction_panic {
            settings::clear_runtime_plane();
            drop(env);
            drop(lock);
            std::panic::resume_unwind(payload);
        }
        super::CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(depth + 1));
        Self {
            home,
            plane,
            env: Some(env),
            lock,
        }
    }

    pub fn config_file_path(&self) -> PathBuf {
        let config_dir = self.home.join(".config");
        let config_dir = match &self.plane {
            TestConfigPlane::Prod | TestConfigPlane::Raw { .. } => config_dir.join("lulu-spark"),
            TestConfigPlane::Sandbox { id } => {
                config_dir.join(settings::SANDBOX_ROOT_DIR_NAME).join(id)
            }
        };
        config_dir.join(settings::PROD_CONFIG_FILE_NAME)
    }

    pub fn ports(&self) -> (u16, u16) {
        match self.plane {
            TestConfigPlane::Prod | TestConfigPlane::Raw { .. } => (
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
        super::CONFIG_TEST_SERIAL_DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
        settings::resync_runtime_plane_after_test_env_restore();
        drop(self.lock.take());
        if let Some(payload) = probe_panic {
            std::panic::resume_unwind(payload);
        }
    }
}
