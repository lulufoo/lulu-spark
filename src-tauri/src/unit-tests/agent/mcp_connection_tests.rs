use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::Method;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use rmcp::{
    ServerHandler,
    model::{
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
    },
    service::RequestContext,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ErrorData as McpError, RoleServer,
};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::ConnectionSlot;
use crate::agent::mcp::McpError as ClientError;
use crate::mcp_host::registry::{HttpMcpTransport, McpServerConfig};

const HANDSHAKE_AND_LIST: [&str; 4] = [
    "initialize",
    "notifications/initialized",
    "tools/list",
    "DELETE",
];

/// Minimal MCP server that exposes one tool.
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
}

/// Records the JSON-RPC method (or HTTP verb) of every request that reaches the server.
async fn record_wire(
    State(wire): State<Arc<Mutex<Vec<String>>>>,
    request: Request,
    next: Next,
) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, 1 << 20).await.expect("request body");
    let label = match parts.method {
        Method::POST => serde_json::from_slice::<Value>(&bytes)
            .ok()
            .and_then(|value| value.get("method").and_then(Value::as_str).map(String::from))
            .unwrap_or_else(|| "POST".into()),
        ref other => other.to_string(),
    };
    wire.lock().expect("wire log").push(label);
    next.run(Request::from_parts(parts, Body::from(bytes))).await
}

struct WireServer {
    port: u16,
    wire: Arc<Mutex<Vec<String>>>,
    stop: CancellationToken,
    thread: Option<JoinHandle<()>>,
}

impl WireServer {
    fn start() -> Self {
        let wire = Arc::new(Mutex::new(Vec::new()));
        let stop = CancellationToken::new();
        let (port_sender, port_receiver) = std::sync::mpsc::channel();
        let thread_wire = wire.clone();
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
                    .layer(middleware::from_fn_with_state(thread_wire, record_wire));
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
            wire,
            stop,
            thread: Some(thread),
        }
    }

    fn config(&self, headers: &[(&str, &str)]) -> McpServerConfig {
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

    /// Handshake / list / delete requests seen so far, in order. The optional
    /// server-event GET stream is not part of what these tests assert.
    fn calls(&self) -> Vec<String> {
        self.wire
            .lock()
            .expect("wire log")
            .iter()
            .filter(|label| HANDSHAKE_AND_LIST.contains(&label.as_str()))
            .cloned()
            .collect()
    }

    fn shut_down(&mut self) {
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

#[test]
fn second_list_tools_reuses_the_connection_without_a_new_handshake() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);

    let first = slot.list_tools(&config).expect("first list");
    let second = slot.list_tools(&config).expect("second list");

    assert!(first.contains("ping") && second.contains("ping"));
    assert_eq!(
        server.calls(),
        ["initialize", "notifications/initialized", "tools/list", "tools/list"],
        "one handshake, two lists, and no DELETE while the connection is live"
    );
}

#[test]
fn changed_config_closes_the_old_connection_and_handshakes_again() {
    let server = WireServer::start();
    let slot = ConnectionSlot::new();

    slot.list_tools(&server.config(&[("x-ticket", "one")]))
        .expect("list with first config");
    slot.list_tools(&server.config(&[("x-ticket", "two")]))
        .expect("list with second config");

    assert_eq!(
        server.calls(),
        [
            "initialize",
            "notifications/initialized",
            "tools/list",
            "DELETE",
            "initialize",
            "notifications/initialized",
            "tools/list",
        ],
        "the first session is deleted before the second one handshakes"
    );
}

#[test]
fn failed_list_drops_the_connection_so_the_next_call_connects_afresh() {
    let mut server = WireServer::start();
    let slot = ConnectionSlot::new();
    let config = server.config(&[]);
    slot.list_tools(&config).expect("list while server is up");

    server.shut_down();

    let on_dead_connection = slot.list_tools(&config).expect_err("list after shutdown");
    assert!(
        matches!(on_dead_connection, ClientError::ListTools(_)),
        "the reused connection fails at tools/list: {on_dead_connection}"
    );
    let on_next_call = slot.list_tools(&config).expect_err("list with no server");
    assert!(
        matches!(on_next_call, ClientError::Connect(_)),
        "the next call must handshake from scratch, not reuse the dead one: {on_next_call}"
    );
}
