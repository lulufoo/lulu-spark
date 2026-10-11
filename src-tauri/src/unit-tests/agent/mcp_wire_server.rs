//! In-process MCP server for connection tests: exposes one tool, records every
//! request that reaches it, and can fail upcoming requests of one method.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use rmcp::{
    ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ErrorData as McpError, RoleServer,
};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::mcp_host::registry::{HttpMcpTransport, McpServerConfig};

/// Requests the connection tests assert on. The optional server-event GET
/// stream is deliberately not part of this list.
const TRACKED_REQUESTS: [&str; 5] = [
    "initialize",
    "notifications/initialized",
    "tools/list",
    "tools/call",
    "DELETE",
];

struct OneToolServer;

impl ServerHandler for OneToolServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        let schema = json!({ "type": "object" })
            .as_object()
            .expect("schema object")
            .clone();
        std::future::ready(Ok(ListToolsResult::with_all_items(vec![Tool::new(
            "ping",
            "Reply with pong",
            Arc::new(schema),
        )])))
    }

    /// `ping` answers `pong`; any other name is a tool-level error result.
    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + Send + '_ {
        let result = if request.name == "ping" {
            CallToolResult::success(vec![ContentBlock::text("pong")])
        } else {
            CallToolResult::error(vec![ContentBlock::text("tool refused")])
        };
        std::future::ready(Ok(result.into()))
    }
}

#[derive(Default)]
struct WireState {
    requests: Mutex<Vec<String>>,
    /// Pending HTTP 500 answers per JSON-RPC method.
    failures: Mutex<HashMap<String, usize>>,
}

/// Records the JSON-RPC method (or HTTP verb) of each request, then either
/// forwards it or answers HTTP 500 when a failure is pending for its method.
async fn record_wire(State(state): State<Arc<WireState>>, request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, 1 << 20).await.expect("request body");
    let label = match parts.method {
        Method::POST => serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|value| value.get("method").and_then(Value::as_str).map(String::from))
            .unwrap_or_else(|| "POST".into()),
        ref other => other.to_string(),
    };
    state.requests.lock().expect("request log").push(label.clone());
    let fail_this_request = {
        let mut failures = state.failures.lock().expect("failure plan");
        match failures.get_mut(&label) {
            Some(pending) if *pending > 0 => {
                *pending -= 1;
                true
            }
            _ => false,
        }
    };
    if fail_this_request {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .expect("error response");
    }
    next.run(Request::from_parts(parts, Body::from(bytes))).await
}

pub struct WireServer {
    port: u16,
    state: Arc<WireState>,
    stop: CancellationToken,
    thread: Option<JoinHandle<()>>,
}

impl WireServer {
    pub fn start() -> Self {
        let state = Arc::new(WireState::default());
        let stop = CancellationToken::new();
        let (port_sender, port_receiver) = std::sync::mpsc::channel();
        let thread_state = state.clone();
        let thread_stop = stop.clone();
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("server runtime");
            runtime.block_on(async move {
                let service: StreamableHttpService<OneToolServer, LocalSessionManager> =
                    StreamableHttpService::new(
                        || Ok(OneToolServer),
                        Arc::new(LocalSessionManager::default()),
                        StreamableHttpServerConfig::default()
                            .with_cancellation_token(thread_stop.child_token()),
                    );
                let router = Router::new()
                    .nest_service("/mcp", service)
                    .layer(middleware::from_fn_with_state(thread_state, record_wire));
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                    .await
                    .expect("bind");
                port_sender
                    .send(listener.local_addr().expect("addr").port())
                    .expect("send port");
                axum::serve(listener, router)
                    .with_graceful_shutdown(thread_stop.cancelled_owned())
                    .await
                    .expect("serve");
            });
        });
        Self {
            port: port_receiver.recv().expect("server port"),
            state,
            stop,
            thread: Some(thread),
        }
    }

    pub fn config(&self, headers: &[(&str, &str)]) -> McpServerConfig {
        McpServerConfig {
            capability_description: "test".into(),
            http_transport: HttpMcpTransport {
                name: "test".into(),
                url: format!("http://127.0.0.1:{}/mcp", self.port),
                headers: headers
                    .iter()
                    .map(|(name, value)| (name.to_string(), value.to_string()))
                    .collect::<BTreeMap<_, _>>(),
            },
        }
    }

    /// Answer the next `count` requests of `method` with HTTP 500.
    // This file is compiled into two test modules; only one injects failures.
    #[allow(dead_code)]
    pub fn fail_next(&self, method: &str, count: usize) {
        self.state
            .failures
            .lock()
            .expect("failure plan")
            .insert(method.to_string(), count);
    }

    /// Tracked requests seen so far, in arrival order.
    pub fn calls(&self) -> Vec<String> {
        self.state
            .requests
            .lock()
            .expect("request log")
            .iter()
            .filter(|label| TRACKED_REQUESTS.contains(&label.as_str()))
            .cloned()
            .collect()
    }

    pub fn shut_down(&mut self) {
        self.stop.cancel();
        if let Some(thread) = self.thread.take() {
            thread.join().expect("server thread");
        }
    }
}

impl Drop for WireServer {
    fn drop(&mut self) {
        self.shut_down();
    }
}
