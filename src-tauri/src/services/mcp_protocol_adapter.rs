//! Host MCP Streamable HTTP listen scaffold (`rmcp`).
//!
//! Binds a localhost listener and nests `StreamableHttpService` under
//! `/mcp/<scene_slot>`. Sidecar `tiny_http` on `:8765` remains separate.
//! MCP runs on an independent OS thread with its own tokio runtime.

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use axum::http::{header, StatusCode};
use axum::routing::get;
use axum::Router;
use rmcp::{
    ServerHandler,
    model::{ServerCapabilities, ServerInfo},
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService,
        session::local::LocalSessionManager,
    },
};
use tokio_util::sync::CancellationToken;

/// Configuration for the embedded Host MCP runtime.
#[derive(Debug, Clone)]
pub struct McpRuntimeConfig {
    pub bind_addr: SocketAddr,
    pub nest_path: String,
}

/// Live MCP runtime handle. Prefer [`stop_embedded_mcp_runtime`] on Host teardown;
/// dropping also cancels and joins the OS thread.
pub struct McpRuntimeHandle {
    local_addr: SocketAddr,
    cancel: CancellationToken,
    join: Option<JoinHandle<()>>,
}

/// Backward-compatible alias from the T1 listen scaffold.
pub type McpListenHandle = McpRuntimeHandle;

impl McpRuntimeHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for McpRuntimeHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Failure starting the Host MCP listen scaffold.
#[derive(Debug)]
pub enum McpStartError {
    BindFailed(String),
    Runtime(String),
}

impl std::fmt::Display for McpStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindFailed(msg) => write!(f, "MCP bind failed: {msg}"),
            Self::Runtime(msg) => write!(f, "MCP runtime error: {msg}"),
        }
    }
}

impl std::error::Error for McpStartError {}

/// Failure stopping the embedded MCP runtime.
#[derive(Debug)]
pub enum McpStopError {
    JoinFailed(String),
}

impl std::fmt::Display for McpStopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JoinFailed(msg) => write!(f, "MCP stop/join failed: {msg}"),
        }
    }
}

impl std::error::Error for McpStopError {}

/// Minimal scaffold handler — tools surface filled in by later tasks.
#[derive(Clone)]
struct ScaffoldHandler;

impl ServerHandler for ScaffoldHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

/// Readiness probe contract matching Node `GET /health` (`ok` + non-empty `mcp`).
async fn health_handler(port: u16) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    let body = serde_json::json!({
        "ok": true,
        "mcp": format!("http://127.0.0.1:{port}/mcp/<scene_slot>"),
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}

/// Start MCP on an independent OS thread with its own tokio runtime.
pub fn start_embedded_mcp_runtime(
    config: McpRuntimeConfig,
) -> Result<McpRuntimeHandle, McpStartError> {
    let nest_path = config.nest_path;
    if nest_path.is_empty() || !nest_path.starts_with('/') {
        return Err(McpStartError::Runtime(format!(
            "nest_path must be an absolute path, got {nest_path:?}"
        )));
    }

    let bind_addr = config.bind_addr;
    let cancel = CancellationToken::new();
    let cancel_child = cancel.child_token();
    let (tx, rx) = std::sync::mpsc::channel::<Result<SocketAddr, String>>();

    let join = thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                let _ = tx.send(Err(format!("tokio runtime: {err}")));
                return;
            }
        };

        rt.block_on(async move {
            let listener = match tokio::net::TcpListener::bind(bind_addr).await {
                Ok(l) => l,
                Err(err) => {
                    let _ = tx.send(Err(err.to_string()));
                    return;
                }
            };
            let local_addr = match listener.local_addr() {
                Ok(addr) => addr,
                Err(err) => {
                    let _ = tx.send(Err(err.to_string()));
                    return;
                }
            };
            if tx.send(Ok(local_addr)).is_err() {
                return;
            }

            let service: StreamableHttpService<ScaffoldHandler, LocalSessionManager> =
                StreamableHttpService::new(
                    || Ok(ScaffoldHandler),
                    Arc::new(LocalSessionManager::default()),
                    StreamableHttpServerConfig::default()
                        .with_cancellation_token(cancel_child.clone()),
                );

            let port = local_addr.port();
            let router = Router::new()
                .route(
                    "/health",
                    get(move || async move { health_handler(port).await }),
                )
                .nest_service(&nest_path, service);
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    cancel_child.cancelled().await;
                })
                .await;
        });
    });

    match rx.recv() {
        Ok(Ok(local_addr)) => Ok(McpRuntimeHandle {
            local_addr,
            cancel,
            join: Some(join),
        }),
        Ok(Err(msg)) => {
            let _ = join.join();
            Err(McpStartError::BindFailed(msg))
        }
        Err(_) => {
            let _ = join.join();
            Err(McpStartError::Runtime(
                "MCP listen thread exited before reporting bind address".into(),
            ))
        }
    }
}

/// Host teardown: cancel the runtime and join the OS thread.
pub fn stop_embedded_mcp_runtime(
    mut handle: McpRuntimeHandle,
) -> Result<(), McpStopError> {
    handle.cancel.cancel();
    if let Some(join) = handle.join.take() {
        join.join()
            .map_err(|_| McpStopError::JoinFailed("MCP runtime thread panicked".into()))?;
    }
    Ok(())
}

/// Bind `bind_addr` and nest Streamable HTTP MCP under `nest_path` (e.g. `/mcp/mvp`).
pub fn start_mcp_listener(
    bind_addr: SocketAddr,
    nest_path: &str,
) -> Result<McpRuntimeHandle, McpStartError> {
    start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr,
        nest_path: nest_path.to_string(),
    })
}

#[cfg(test)]
#[path = "../unit-tests/services/mcp_protocol_adapter_tests.rs"]
mod tests;
