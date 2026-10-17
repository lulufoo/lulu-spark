use std::path::Path;

use super::runtime_mode::{runtime_mode, runtime_normalize_prod_paths, RuntimeMode};
use super::types::{
    default_cache_dir, default_spark_root, AppSettings, SettingsError, DEFAULT_PROD_GATEWAY_PORT, DEFAULT_PROD_HTTP_PORT,
    DEFAULT_PROD_MCP_PORT, PROD_CONFIG_FILE_NAME,
};

pub(super) fn path_equals_or_under(path: &Path, root: &Path) -> bool {
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
    let mode = runtime_mode()?;
    if !matches!(mode, RuntimeMode::Sandbox { .. }) {
        return Ok(());
    }
    for (label, path, prod_root) in [
        (
            "spark_root",
            sandbox.spark_root.as_path(),
            prod.spark_root.as_path(),
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
    let sh = sandbox.effective_http_port()?;
    let sm = sandbox.effective_mcp_port()?;
    let sg = sandbox.effective_gateway_port()?;
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

/// Err when the runtime plane is unreadable; callers then skip normalize.
pub(super) fn should_normalize_for_path(path: &Path) -> Result<bool, SettingsError> {
    // Boot installs whether prod-path healing is on; sandbox plane keeps written roots.
    if !runtime_normalize_prod_paths()? {
        return Ok(false);
    }
    if path.ancestors().any(is_unstable_path) {
        return Ok(false);
    }
    Ok(path == super::store::prod_config_file_path() && config_is_prod_file(path))
}

pub(crate) fn normalize_prod_paths(settings: &mut AppSettings) {
    if is_unstable_cache_dir(&settings.cache_dir) {
        settings.cache_dir = default_cache_dir();
    }
    if is_unstable_path(&settings.spark_root) {
        settings.spark_root = default_spark_root();
    }
}

/// Reset poisoned `cache_dir` values (e.g. test temp paths written via `set_config`).
pub(crate) fn normalize_cache_dir(settings: &mut AppSettings) {
    if is_unstable_cache_dir(&settings.cache_dir) {
        settings.cache_dir = default_cache_dir();
    }
}
