//! `cfg(test)`-only config guards: machine config protection, atomic test writes,
//! and the temp-HOME check behind `install_runtime_plane` in Prod mode.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::runtime_mode::resolve_real_user_home;
use super::types::{home_dir, SettingsError};

static TEST_MACHINE_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Record the machine config path before a test fixture changes `HOME`.
pub(crate) fn register_test_machine_config_path(path: PathBuf) {
    if let Some(registered) = TEST_MACHINE_CONFIG_PATH.get() {
        assert_eq!(
            registered, &path,
            "test machine config path changed during one test process"
        );
        return;
    }
    if let Err(registered) = TEST_MACHINE_CONFIG_PATH.set(path.clone()) {
        assert_eq!(
            registered, path,
            "test machine config path changed during one test process"
        );
    }
}

enum GuardPathComponent {
    Normal(OsString),
    Parent,
}

fn canonical_path_or_existing_parent(path: &Path) -> Result<PathBuf, SettingsError> {
    match fs::canonicalize(path) {
        Ok(resolved) => return Ok(resolved),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(SettingsError::ConfigGuard(format!(
                "cannot resolve config path {}: {error}",
                path.display()
            )));
        }
    }

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(SettingsError::Io)?
            .join(path)
    };
    let mut probe = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        match fs::canonicalize(probe) {
            Ok(mut resolved) => {
                for component in suffix.iter().rev() {
                    match component {
                        GuardPathComponent::Normal(name) => resolved.push(name),
                        GuardPathComponent::Parent => {
                            if !resolved.pop() {
                                return Err(SettingsError::ConfigGuard(format!(
                                    "config path escapes filesystem root: {}",
                                    path.display()
                                )));
                            }
                        }
                    }
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(SettingsError::ConfigGuard(format!(
                    "cannot resolve config path {}: {error}",
                    path.display()
                )));
            }
        }
        let component = probe.components().next_back().ok_or_else(|| {
            SettingsError::ConfigGuard(format!("cannot resolve config path {}", path.display()))
        })?;
        match component {
            std::path::Component::Normal(name) => {
                suffix.push(GuardPathComponent::Normal(name.to_os_string()));
            }
            std::path::Component::ParentDir => suffix.push(GuardPathComponent::Parent),
            std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(SettingsError::ConfigGuard(format!(
                    "cannot resolve config path {}",
                    path.display()
                )));
            }
        }
        probe = probe.parent().ok_or_else(|| {
            SettingsError::ConfigGuard(format!("cannot resolve config path {}", path.display()))
        })?;
    }
}

/// Reject an alias that resolves to a protected formal config path.
pub(crate) fn reject_write_to_protected_config(
    path: &Path,
    protected_config_path: &Path,
) -> Result<(), SettingsError> {
    let resolved_path = canonical_path_or_existing_parent(path)?;
    let resolved_protected = canonical_path_or_existing_parent(protected_config_path)?;
    if resolved_path == resolved_protected {
        return Err(SettingsError::ConfigGuard(format!(
            "test config write targets protected machine config: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Reject test writes until the machine formal config path is registered.
pub(crate) fn reject_test_write_to_machine_config(path: &Path) -> Result<(), SettingsError> {
    let protected = TEST_MACHINE_CONFIG_PATH.get().ok_or_else(|| {
        SettingsError::ConfigGuard(
            "machine formal config path was not registered before test write".into(),
        )
    })?;
    reject_write_to_protected_config(path, protected)
}

/// Atomically replace an already-verified test config target.
pub(crate) fn atomic_write_test_config(
    verified_target_path: &Path,
    content: &str,
) -> Result<(), SettingsError> {
    let parent = verified_target_path.parent().ok_or_else(|| {
        SettingsError::ConfigGuard(format!(
            "test config path has no parent: {}",
            verified_target_path.display()
        ))
    })?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(content.as_bytes())?;
    temporary.flush()?;
    temporary
        .persist(verified_target_path)
        .map_err(|error| SettingsError::Io(error.error))?;
    Ok(())
}

pub fn write_test_config_with_cache(
    config_path: &Path,
    spark_root: &Path,
    cache_dir: Option<&Path>,
    http_port: u16,
    mcp_port: u16,
) -> Result<(), SettingsError> {
    let cache = cache_dir
        .map(|p| p.to_path_buf())
        .or_else(|| config_path.parent().map(|parent| parent.join("cache")))
        .ok_or_else(|| {
            SettingsError::ConfigGuard(format!(
                "test config path has no parent: {}",
                config_path.display()
            ))
        })?;
    reject_test_write_to_machine_config(config_path)?;
    fs::create_dir_all(&cache)?;
    let text = format!(
        "spark_root = \"{}\"\ncache_dir = \"{}\"\nhttp_port = {http_port}\nmcp_port = {mcp_port}\n",
        spark_root.display(),
        cache.display()
    );
    atomic_write_test_config(config_path, &text)
}

/// Lib tests must never resolve Prod data paths against the real user home.
pub(crate) fn assert_test_home_isolated() {
    let home = home_dir();
    if resolve_real_user_home().is_some_and(|real| real == home) {
        panic!(
            "cargo test resolved Prod cache dir under the real HOME {}; open a TestSandbox",
            home.display()
        );
    }
}

#[path = "../../unit-tests/config/settings/sandbox_test_guard.rs"]
mod tests;
