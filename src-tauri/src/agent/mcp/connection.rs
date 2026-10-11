//! One live MCP connection per server config, shared by the Host Agent.
//!
//! The server keeps a stateful session per connection, so reuse means keeping
//! the live client, not just remembering that a handshake happened. The
//! connection lives on a process-wide runtime because rmcp's background tasks
//! stop when the runtime that spawned them is dropped.

use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, CallToolResult, ClientInfo, PaginatedRequestParams},
    service::{Peer, RoleClient, RunningService},
    transport::StreamableHttpClientTransport,
};
use tokio::runtime::{Builder, Runtime};

use crate::mcp_host::registry::McpServerConfig;

use crate::agent::tools::catalog::{self, ToolCatalog};

use super::connection_log::{McpTrace, McpTurn, ResetReason};
use super::{client_info, transport_config, McpError};

type Client = RunningService<RoleClient, ClientInfo>;

static RUNTIME: LazyLock<Result<Runtime, String>> = LazyLock::new(|| {
    Builder::new_multi_thread()
        .worker_threads(2)
        .thread_name("spark-mcp-client")
        .enable_all()
        .build()
        .map_err(|error| error.to_string())
});

fn runtime() -> Result<&'static Runtime, McpError> {
    RUNTIME
        .as_ref()
        .map_err(|error| McpError::Runtime(error.clone()))
}

struct LiveConnection {
    /// Config the client was created with; any difference means a new connection.
    config: McpServerConfig,
    client: Client,
}

/// Holder of the live connection. Production uses [`CONNECTION`]; tests build
/// their own so parallel tests never share one.
pub(super) struct ConnectionSlot {
    live: Mutex<Option<LiveConnection>>,
}

/// The Host Agent's connection to its MCP server.
pub(super) static CONNECTION: ConnectionSlot = ConnectionSlot::new();

impl ConnectionSlot {
    pub(super) const fn new() -> Self {
        Self {
            live: Mutex::new(None),
        }
    }

    /// Discover the model-callable tools over the live connection, handshaking
    /// first when there is none or `config` changed.
    ///
    /// A failed request drops the connection. When the failed connection was a
    /// reused one it may simply be stale, so handshake once more and retry once;
    /// a connection that was just created is not retried.
    pub(super) fn list_tools(
        &self,
        config: &McpServerConfig,
        turn: &McpTurn,
    ) -> Result<ToolCatalog, McpError> {
        let (peer, reused) = self.peer_for(config, turn)?;
        let started = Instant::now();
        let (result, retried) = match (self.fetch_tools(&peer, turn), reused) {
            (Err(_), true) => (
                self.peer_for(config, turn)
                    .and_then(|(peer, _)| self.fetch_tools(&peer, turn)),
                true,
            ),
            (outcome, _) => (outcome, false),
        };
        turn.list_completed(started.elapsed(), reused, retried, &result);
        result
    }

    /// Run one `tools/call` over the live connection, handshaking first when
    /// there is none. Any error from the request drops the connection so the
    /// next request handshakes again; a tool that merely reports `is_error` is
    /// a successful call and keeps it. The failed call itself is not retried.
    pub(super) fn call_tool(
        &self,
        config: &McpServerConfig,
        mut params: CallToolRequestParams,
        trace: &McpTrace,
    ) -> Result<CallToolResult, McpError> {
        params.meta = Some(trace.meta());
        let (peer, _) = self.peer_for(config, &trace.turn())?;
        let result = runtime()?
            .block_on(peer.call_tool(params))
            .map_err(|error| McpError::CallTool(format!("{error:#}")));
        if result.is_err() {
            self.reset(trace, ResetReason::CallFailed);
        }
        result
    }

    /// Peer of the live connection for `config`, and whether it was reused
    /// rather than freshly handshaken.
    fn peer_for(
        &self,
        config: &McpServerConfig,
        turn: &McpTurn,
    ) -> Result<(Peer<RoleClient>, bool), McpError> {
        let mut live = self.live.lock().unwrap_or_else(PoisonError::into_inner);
        let reused = live
            .as_ref()
            .is_some_and(|connection| connection.config == *config);
        if !reused {
            if let Some(stale) = live.take() {
                turn.connection_reset(ResetReason::ConfigChanged);
                close(stale);
            }
            let client = connect(config, turn)?;
            *live = Some(LiveConnection {
                config: config.clone(),
                client,
            });
        }
        live.as_ref()
            .map(|connection| (connection.client.peer().clone(), reused))
            .ok_or_else(|| McpError::Connect("connection missing after connect".into()))
    }

    /// One `tools/list` request; a failure drops the connection.
    fn fetch_tools(&self, peer: &Peer<RoleClient>, turn: &McpTurn) -> Result<ToolCatalog, McpError> {
        let trace = turn.trace();
        let mut params = PaginatedRequestParams::default();
        params.meta = Some(trace.meta());
        let result = runtime()?
            .block_on(peer.list_tools(Some(params)))
            .map_err(|error| McpError::ListTools(format!("{error:#}")));
        match result {
            Ok(tools) => Ok(catalog::from_rmcp_tools(tools.tools)),
            Err(error) => {
                self.reset(&trace, ResetReason::ListFailed);
                Err(error)
            }
        }
    }

    /// Drop the live connection, if any, and close its server session.
    pub(super) fn reset(&self, trace: &McpTrace, reason: ResetReason) {
        let stale = self
            .live
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(stale) = stale {
            trace.connection_reset(reason);
            close(stale);
        }
    }
}

fn connect(config: &McpServerConfig, turn: &McpTurn) -> Result<Client, McpError> {
    let trace = turn.trace();
    let transport_config = transport_config(config)?;
    trace.connect_started();
    let started = Instant::now();
    // The transport spawns its worker on construction, so it must be built
    // inside the runtime that will keep serving the connection.
    let mut info = client_info();
    info.meta = Some(trace.meta());
    let client = runtime()?
        .block_on(async move {
            let transport = StreamableHttpClientTransport::from_config(transport_config);
            info.serve(transport).await
        })
        .map_err(|error| McpError::Connect(format!("{error:#}")));
    trace.connect_completed(started.elapsed(), client.is_ok());
    client
}

fn close(connection: LiveConnection) {
    if let Ok(runtime) = runtime() {
        let _ = runtime.block_on(connection.client.cancel());
    }
}

#[cfg(test)]
#[path = "../../unit-tests/agent/mcp_connection_tests.rs"]
mod tests;
