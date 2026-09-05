use std::sync::Mutex;

/// Holds the embedded Host MCP runtime (independent OS thread + tokio).
/// Lifecycle: start in app setup; stop/join on app exit.
pub struct EmbeddedMcpRuntime {
    handle: Mutex<Option<crate::mcp_host::McpRuntimeHandle>>,
}

impl EmbeddedMcpRuntime {
    pub fn new(handle: Option<crate::mcp_host::McpRuntimeHandle>) -> Self {
        Self {
            handle: Mutex::new(handle),
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                let _ = crate::mcp_host::stop_embedded_mcp_runtime(handle);
            }
        }
    }
}
