//! Host MCP Streamable HTTP listen scaffold (`rmcp`).
//!
//! Binds a localhost listener and nests `StreamableHttpService` under
//! `/mcp/<scene_slot>`. Sidecar `tiny_http` on `:8765` remains separate.

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

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

/// Live MCP listen handle. Dropping cancels the runtime and joins the OS thread.
pub struct McpListenHandle {
    local_addr: SocketAddr,
    cancel: CancellationToken,
    join: Option<JoinHandle<()>>,
}

impl McpListenHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for McpListenHandle {
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

/// Minimal scaffold handler — tools surface filled in by later tasks.
#[derive(Clone)]
struct ScaffoldHandler;

impl ServerHandler for ScaffoldHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

/// Bind `bind_addr` and nest Streamable HTTP MCP under `nest_path` (e.g. `/mcp/mvp`).
pub fn start_mcp_listener(
    bind_addr: SocketAddr,
    nest_path: &str,
) -> Result<McpListenHandle, McpStartError> {
    if nest_path.is_empty() || !nest_path.starts_with('/') {
        return Err(McpStartError::Runtime(format!(
            "nest_path must be an absolute path, got {nest_path:?}"
        )));
    }

    let nest_path = nest_path.to_string();
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

            let router = Router::new().nest_service(&nest_path, service);
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    cancel_child.cancelled().await;
                })
                .await;
        });
    });

    match rx.recv() {
        Ok(Ok(local_addr)) => Ok(McpListenHandle {
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

#[cfg(test)]
#[path = "../unit-tests/services/mcp_protocol_adapter_tests.rs"]
mod tests;
