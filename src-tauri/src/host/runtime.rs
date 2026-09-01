use std::sync::Mutex;

use crate::services;

/// Holds the spawned Meilisearch child process so we can kill it on app exit.
#[cfg(not(test))]
pub struct MeiliProcess(Mutex<Option<std::process::Child>>);

#[cfg(not(test))]
impl MeiliProcess {
    pub(crate) fn new(child: Option<std::process::Child>) -> Self {
        Self(Mutex::new(child))
    }

    pub fn kill(&self) {
        if let Ok(mut guard) = self.0.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

/// Holds the embedded Host MCP runtime (independent OS thread + tokio).
/// Lifecycle: start in app setup; stop/join on app exit.
pub struct EmbeddedMcpRuntime {
    handle: Mutex<Option<services::mcp_host::McpRuntimeHandle>>,
}

impl EmbeddedMcpRuntime {
    pub fn new(handle: Option<services::mcp_host::McpRuntimeHandle>) -> Self {
        Self {
            handle: Mutex::new(handle),
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = services::mcp_host::stop_embedded_mcp_runtime(handle);
            }
        }
    }
}

/// Try to start the system `meilisearch` binary if the configured port is not
/// already listening. Silently skips if the binary is not installed or the port
/// is already up (e.g. user runs Meilisearch as a service).
#[cfg(not(test))]
pub(crate) fn try_autostart_meilisearch() -> Option<std::process::Child> {
    use std::time::Duration;

    use crate::config::{secrets, settings};
    use super::port::{decide_spawn, wait_for_port, SpawnDecision};

    let cfg = settings::load().unwrap_or_default();

    let port: u16 = cfg
        .meili_url
        .trim_end_matches('/')
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7700);

    if decide_spawn(port, Duration::from_millis(0)) == SpawnDecision::Skip {
        eprintln!("[meili] port {port} already listening — skip autostart");
        return None;
    }

    let master_key = secrets::get_secret(secrets::KEY_MEILI_MASTER)
        .ok()
        .flatten()
        .unwrap_or_else(|| "lulu-workbench-local".to_string());

    let db_path = cfg.cache_dir.join(".meilisearch");
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match std::process::Command::new("meilisearch")
        .args([
            "--no-analytics",
            "--db-path",
            &db_path.to_string_lossy(),
            "--master-key",
            &master_key,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            eprintln!("[meili] spawned pid={}", child.id());
            if wait_for_port(port, Duration::from_secs(5)) {
                eprintln!("[meili] ready on port {port}");
            } else {
                eprintln!("[meili] warn: port {port} not ready after 5 s");
            }
            Some(child)
        }
        Err(e) => {
            eprintln!("[meili] autostart skipped ({e}); install with: brew install meilisearch");
            None
        }
    }
}
