//! Single entry for the runtime plane (Prod vs TestSandbox) and the cache root.
//!
//! Boot (and test fixtures) call [`install_runtime_plane`] once after the process
//! env is set; runtime path helpers then read [`runtime_config_dir`] /
//! [`runtime_cache_dir`] from the process plane — they never re-resolve env
//! or fall back to prod paths.

use std::path::PathBuf;
use std::sync::Mutex;

use super::types::{
    default_cache_dir, home_dir, validate_test_sandbox_id, SettingsError, ENV_TEST_SANDBOX,
    ENV_TEST_SANDBOX_ID, DEFAULT_PROD_GATEWAY_PORT, DEFAULT_PROD_HTTP_PORT, DEFAULT_PROD_MCP_PORT,
    DEFAULT_SANDBOX_GATEWAY_PORT, DEFAULT_SANDBOX_HTTP_PORT, DEFAULT_SANDBOX_MCP_PORT,
};

/// Sandbox instances live under `<HOME>/.config/lulu-spark-sandbox/<id>/`.
pub const SANDBOX_ROOT_DIR_NAME: &str = "lulu-spark-sandbox";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMode {
    Prod,
    Sandbox { id: String },
}

#[derive(Debug, Clone)]
struct RuntimePlane {
    mode: RuntimeMode,
    /// Active config root (prod `~/.config/lulu-spark` or sandbox base dir).
    config_dir: PathBuf,
    cache_dir: PathBuf,
    /// When true, load/save may heal unstable roots to prod defaults.
    normalize_prod_paths: bool,
    http_port: u16,
    mcp_port: u16,
    gateway_port: u16,
}

/// Mode-default listen ports. Only used when installing the plane at boot/fixture.
fn default_listen_ports(mode: &RuntimeMode) -> (u16, u16, u16) {
    match mode {
        RuntimeMode::Prod => (
            DEFAULT_PROD_HTTP_PORT,
            DEFAULT_PROD_MCP_PORT,
            DEFAULT_PROD_GATEWAY_PORT,
        ),
        RuntimeMode::Sandbox { .. } => (
            DEFAULT_SANDBOX_HTTP_PORT,
            DEFAULT_SANDBOX_MCP_PORT,
            DEFAULT_SANDBOX_GATEWAY_PORT,
        ),
    }
}

static RUNTIME_PLANE: Mutex<Option<RuntimePlane>> = Mutex::new(None);

fn plane_slot() -> std::sync::MutexGuard<'static, Option<RuntimePlane>> {
    RUNTIME_PLANE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Read `TestSandbox` / `TestSandboxId` from the process env (pure; no plane write).
///
/// Unset → `Prod`; `true` / `1` with a valid id → `Sandbox`; anything else → `Err`.
fn resolve_runtime_mode() -> Result<RuntimeMode, SettingsError> {
    let flag = std::env::var(ENV_TEST_SANDBOX).ok();
    match flag.as_deref() {
        None => Ok(RuntimeMode::Prod),
        Some("true") | Some("1") => {
            let id = std::env::var(ENV_TEST_SANDBOX_ID)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    SettingsError::ConfigGuard("TestSandbox set but TestSandboxId missing".into())
                })?;
            validate_test_sandbox_id(&id)?;
            Ok(RuntimeMode::Sandbox { id })
        }
        Some(other) => Err(SettingsError::ConfigGuard(format!(
            "invalid TestSandbox value: {other:?} (need true or 1)"
        ))),
    }
}

/// `<HOME>/.config/lulu-spark-sandbox/<id>`.
pub fn resolve_sandbox_dir(id: &str) -> PathBuf {
    home_dir().join(".config").join(SANDBOX_ROOT_DIR_NAME).join(id)
}

/// Compute the cache root from the current env (pure; no plane write).
///
/// Sandbox → `<sandbox_dir>/cache`; Prod → `default_cache_dir()`.
fn resolve_cache_dir() -> Result<PathBuf, SettingsError> {
    match resolve_runtime_mode()? {
        RuntimeMode::Prod => Ok(default_cache_dir()),
        RuntimeMode::Sandbox { id } => Ok(resolve_sandbox_dir(&id).join("cache")),
    }
}

/// Resolve mode + cache dir from env and store them as the process plane.
///
/// Call once at boot after env is final. Sandbox resolve failure → `Err` (refuse start).
/// Under `cfg(test)`, Prod refuses the real user HOME so lib tests cannot touch it.
pub fn install_runtime_plane() -> Result<RuntimeMode, SettingsError> {
    let mode = resolve_runtime_mode()?;
    #[cfg(test)]
    if matches!(mode, RuntimeMode::Prod) {
        super::sandbox_test_guard::assert_test_home_isolated();
    }
    let config_dir = match &mode {
        RuntimeMode::Prod => home_dir().join(".config").join("lulu-spark"),
        RuntimeMode::Sandbox { id } => resolve_sandbox_dir(id),
    };
    let cache_dir = resolve_cache_dir()?;
    let normalize_prod_paths = matches!(mode, RuntimeMode::Prod);
    let (http_port, mcp_port, gateway_port) = default_listen_ports(&mode);
    *plane_slot() = Some(RuntimePlane {
        mode: mode.clone(),
        config_dir,
        cache_dir,
        normalize_prod_paths,
        http_port,
        mcp_port,
        gateway_port,
    });
    Ok(mode)
}

/// Overwrite listen ports on the installed plane (boot after loading settings).
pub fn install_runtime_listen_ports(http: u16, mcp: u16, gateway: u16) -> Result<(), SettingsError> {
    let mut slot = plane_slot();
    let plane = slot.as_mut().ok_or_else(|| {
        SettingsError::ConfigGuard("runtime plane not initialized".into())
    })?;
    plane.http_port = http;
    plane.mcp_port = mcp;
    plane.gateway_port = gateway;
    Ok(())
}

/// Drop the process plane (test fixtures after env restore).
pub fn clear_runtime_plane() {
    *plane_slot() = None;
}

/// After a test fixture applies env: install, or clear when resolve fails (invalid Raw).
pub fn sync_runtime_plane_from_env() {
    match install_runtime_plane() {
        Ok(_) => {}
        Err(_) => clear_runtime_plane(),
    }
}

/// After a test fixture restores env: re-install from the restored env, or clear.
///
/// Real-user HOME in Prod clears instead of installing — lib tests must not
/// adopt the machine plane (and must not panic via the Prod HOME guard).
pub fn resync_runtime_plane_after_test_env_restore() {
    match resolve_runtime_mode() {
        Ok(mode) => {
            #[cfg(test)]
            if matches!(mode, RuntimeMode::Prod) {
                let home = home_dir();
                if resolve_real_user_home().is_some_and(|real| real == home) {
                    clear_runtime_plane();
                    return;
                }
            }
            match install_runtime_plane() {
                Ok(_) => {}
                Err(_) => clear_runtime_plane(),
            }
        }
        Err(_) => clear_runtime_plane(),
    }
}

/// Cached mode from the process plane.
pub fn runtime_mode() -> Result<RuntimeMode, SettingsError> {
    plane_slot()
        .as_ref()
        .map(|p| p.mode.clone())
        .ok_or_else(|| SettingsError::ConfigGuard("runtime plane not initialized".into()))
}

/// Active config directory from the process plane.
pub fn runtime_config_dir() -> Result<PathBuf, SettingsError> {
    plane_slot()
        .as_ref()
        .map(|p| p.config_dir.clone())
        .ok_or_else(|| SettingsError::ConfigGuard("runtime plane not initialized".into()))
}

/// Cached cache root from the process plane. Never falls back to prod defaults.
pub fn runtime_cache_dir() -> Result<PathBuf, SettingsError> {
    plane_slot()
        .as_ref()
        .map(|p| p.cache_dir.clone())
        .ok_or_else(|| SettingsError::ConfigGuard("runtime plane not initialized".into()))
}

/// Whether load/save should normalize unstable roots to prod defaults.
pub fn runtime_normalize_prod_paths() -> Result<bool, SettingsError> {
    plane_slot()
        .as_ref()
        .map(|p| p.normalize_prod_paths)
        .ok_or_else(|| SettingsError::ConfigGuard("runtime plane not initialized".into()))
}

/// HTTP listen port from the installed plane.
pub fn runtime_http_port() -> Result<u16, SettingsError> {
    plane_slot()
        .as_ref()
        .map(|p| p.http_port)
        .ok_or_else(|| SettingsError::ConfigGuard("runtime plane not initialized".into()))
}

/// MCP listen port from the installed plane.
pub fn runtime_mcp_port() -> Result<u16, SettingsError> {
    plane_slot()
        .as_ref()
        .map(|p| p.mcp_port)
        .ok_or_else(|| SettingsError::ConfigGuard("runtime plane not initialized".into()))
}

/// Gateway listen port from the installed plane.
pub fn runtime_gateway_port() -> Result<u16, SettingsError> {
    plane_slot()
        .as_ref()
        .map(|p| p.gateway_port)
        .ok_or_else(|| SettingsError::ConfigGuard("runtime plane not initialized".into()))
}

/// Real user home from the passwd database (ignores the `HOME` env var).
pub fn resolve_real_user_home() -> Option<PathBuf> {
    use std::ffi::{CStr, OsStr};
    use std::os::unix::ffi::OsStrExt;
    // SAFETY: getpwuid returns a pointer into static storage or null; read once, copy out.
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_dir.is_null() {
            return None;
        }
        let raw = CStr::from_ptr((*pw).pw_dir).to_bytes();
        Some(PathBuf::from(OsStr::from_bytes(raw)))
    }
}

#[cfg(test)]
#[path = "../../unit-tests/config/settings/runtime_mode.rs"]
mod tests;
