use std::path::Path;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
use std::ffi::OsString;
#[cfg(test)]
use std::io::Write;
#[cfg(test)]
use std::sync::OnceLock;

use super::types::{
    default_cache_dir, default_knowledge_root, default_spark_root,
    is_test_sandbox, AppSettings, SettingsError, DEFAULT_PROD_GATEWAY_PORT, DEFAULT_PROD_HTTP_PORT,
    DEFAULT_PROD_MCP_PORT, PROD_CONFIG_FILE_NAME,
};

#[cfg(test)]
static TEST_MACHINE_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Record the machine config path before a test fixture changes `HOME`.
#[cfg(test)]
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

fn path_equals_or_under(path: &Path, root: &Path) -> bool {
    if path == root {
        return true;
    }
    path.starts_with(root)
}

/// Fail-closed sandbox guard: roots / ports must not collide with prod.
pub fn validate_sandbox_against_prod(
    sandbox: &AppSettings,
    prod: &AppSettings,
) -> Result<(), SettingsError> {
    if !is_test_sandbox() {
        return Ok(());
    }
    for (label, path, prod_root) in [
        (
            "spark_root",
            sandbox.spark_root.as_path(),
            prod.spark_root.as_path(),
        ),
        (
            "knowledge_root",
            sandbox.knowledge_root.as_path(),
            prod.knowledge_root.as_path(),
        ),
        (
            "cache_dir",
            sandbox.cache_dir.as_path(),
            prod.cache_dir.as_path(),
        ),
    ] {
        if path.as_os_str().is_empty() {
            return Err(SettingsError::ConfigGuard(format!(
                "sandbox required field empty: {label}"
            )));
        }
        if path_equals_or_under(path, prod_root) {
            return Err(SettingsError::ConfigGuard(format!(
                "sandbox {label} collides with prod root {}",
                prod_root.display()
            )));
        }
    }
    let sh = sandbox.effective_http_port();
    let sm = sandbox.effective_mcp_port();
    let sg = sandbox.effective_gateway_port();
    // Prod ports must not use sandbox plane defaults (env may be TestSandbox=true here).
    let ph = prod.http_port.unwrap_or(DEFAULT_PROD_HTTP_PORT);
    let pm = prod.mcp_port.unwrap_or(DEFAULT_PROD_MCP_PORT);
    let pg = prod.gateway_port.unwrap_or(DEFAULT_PROD_GATEWAY_PORT);
    for s in [sh, sm, sg] {
        for p in [ph, pm, pg] {
            if s == p {
                return Err(SettingsError::ConfigGuard(format!(
                    "sandbox ports collide with prod (sandbox http={sh} mcp={sm} gateway={sg}, prod http={ph} mcp={pm} gateway={pg})"
                )));
            }
        }
    }
    Ok(())
}

/// True when path points at OS/tempfile ephemeral storage (must not persist in prod config).
pub(crate) fn is_unstable_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.starts_with("/tmp/") || s == "/tmp" {
        return true;
    }
    // macOS `$TMPDIR`: `/var/folders/.../T/.tmpXXXXXX/...`
    if s.contains("/T/.tmp") {
        return true;
    }
    for comp in path.components() {
        if let std::path::Component::Normal(name) = comp {
            let n = name.to_string_lossy();
            if n.starts_with(".tmp") && n.len() > 4 {
                return true;
            }
        }
    }
    false
}

pub(crate) fn is_unstable_cache_dir(path: &Path) -> bool {
    is_unstable_path(path)
}

fn config_is_prod_file(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some(PROD_CONFIG_FILE_NAME)
}

pub(super) fn should_normalize_for_path(path: &Path) -> bool {
    // Sandbox / temp HOME fixtures must keep written roots (do not "heal" to ~/Code).
    if is_test_sandbox() {
        return false;
    }
    if path.ancestors().any(is_unstable_path) {
        return false;
    }
    path == super::store::prod_config_file_path() && config_is_prod_file(path)
}

pub(crate) fn normalize_prod_paths(settings: &mut AppSettings) {
    if is_unstable_cache_dir(&settings.cache_dir) {
        settings.cache_dir = default_cache_dir();
    }
    if is_unstable_path(&settings.spark_root) {
        settings.spark_root = default_spark_root();
    }
    if is_unstable_path(&settings.knowledge_root) {
        settings.knowledge_root = default_knowledge_root();
    }
}

/// Reset poisoned `cache_dir` values (e.g. test temp paths written via `set_config`).
pub(crate) fn normalize_cache_dir(settings: &mut AppSettings) {
    if is_unstable_cache_dir(&settings.cache_dir) {
        settings.cache_dir = default_cache_dir();
    }
}

#[cfg(test)]
enum GuardPathComponent {
    Normal(OsString),
    Parent,
}

#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
pub(crate) fn reject_test_write_to_machine_config(path: &Path) -> Result<(), SettingsError> {
    let protected = TEST_MACHINE_CONFIG_PATH.get().ok_or_else(|| {
        SettingsError::ConfigGuard(
            "machine formal config path was not registered before test write".into(),
        )
    })?;
    reject_write_to_protected_config(path, protected)
}

/// Atomically replace an already-verified test config target.
#[cfg(test)]
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

#[cfg(test)]
pub fn write_test_config_with_cache(
    config_path: &Path,
    spark_root: &Path,
    knowledge_root: Option<&Path>,
    cache_dir: Option<&Path>,
    http_port: u16,
    mcp_port: u16,
) -> Result<(), SettingsError> {
    let knowledge = knowledge_root
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| spark_root.display().to_string());
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
        "spark_root = \"{}\"\nknowledge_root = \"{}\"\ncache_dir = \"{}\"\nhttp_port = {http_port}\nmcp_port = {mcp_port}\n",
        spark_root.display(),
        knowledge,
        cache.display()
    );
    atomic_write_test_config(config_path, &text)
}
