//! Startup settings: install the runtime plane first, then load config and listen ports.

use crate::config::settings::{self, AppSettings, RuntimeMode};

pub struct BootSettings {
    pub mode: RuntimeMode,
    pub settings: AppSettings,
    pub http_port: u16,
    pub mcp_port: u16,
    pub gateway_port: u16,
}

/// `Err` means refuse to start: unreadable runtime mode, or a sandbox whose config fails.
/// Prod keeps its fallback to default settings when `config.toml` cannot load.
pub fn resolve_boot_settings() -> Result<BootSettings, String> {
    let mode = settings::install_runtime_plane().map_err(|e| format!("runtime mode: {e}"))?;
    let loaded = match settings::load() {
        Ok(s) => s,
        Err(err) => {
            eprintln!("[settings] load failed: {err}");
            if matches!(mode, RuntimeMode::Sandbox { .. }) {
                return Err(format!("sandbox settings: {err}"));
            }
            AppSettings::default()
        }
    };
    let http_port = loaded.effective_http_port().map_err(|e| format!("{e}"))?;
    let mcp_port = loaded.effective_mcp_port().map_err(|e| format!("{e}"))?;
    let gateway_port = loaded.effective_gateway_port().map_err(|e| format!("{e}"))?;
    settings::install_runtime_listen_ports(http_port, mcp_port, gateway_port)
        .map_err(|e| format!("runtime ports: {e}"))?;
    Ok(BootSettings {
        http_port,
        mcp_port,
        gateway_port,
        mode,
        settings: loaded,
    })
}

#[cfg(test)]
#[path = "unit-tests/boot_settings.rs"]
mod tests;
