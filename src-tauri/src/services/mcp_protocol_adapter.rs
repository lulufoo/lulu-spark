//! Host MCP Streamable HTTP listen (`rmcp`).
//!
//! Binds a localhost listener and nests `StreamableHttpService` under
//! `/mcp/<scene_slot>` for registered slots only. Unknown slots are rejected
//! at the HTTP/routing layer without creating an MCP session.
//! Sidecar `tiny_http` on `:8765` remains separate.
//! Adapter proxies tools only via HTTP loopback to `127.0.0.1:8765/api/*`.
//! MCP runs on an independent OS thread with its own tokio runtime.

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{any, get};
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
use serde_json::Value;
use tokio_util::sync::CancellationToken;

/// Default Sidecar loopback base (Host tiny_http `:8765`).
pub const DEFAULT_SIDECAR_BASE_URL: &str = "http://127.0.0.1:8765";

/// Observable MCP tool success (Node: `{ content: [{ type: "text", text }] }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolResult {
    pub content_text: String,
}

/// Observable MCP tool error (Node: `{ content: [{ type: "text", text: "HTTP …" }], isError: true }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolError {
    pub content_text: String,
}

/// Map Sidecar HTTP status+body to Node-equivalent MCP tool result/error.
pub fn map_sidecar_response_to_mcp(
    http_status: u16,
    body: &[u8],
) -> Result<McpToolResult, McpToolError> {
    let text = String::from_utf8_lossy(body).into_owned();
    if (200..300).contains(&http_status) {
        Ok(McpToolResult {
            content_text: text,
        })
    } else {
        Err(McpToolError {
            content_text: format!("HTTP {http_status}: {text}"),
        })
    }
}

/// Convert mapped success/error into `rmcp` [`CallToolResult`].
pub fn mapped_to_call_tool_result(
    mapped: Result<McpToolResult, McpToolError>,
) -> CallToolResult {
    match mapped {
        Ok(ok) => CallToolResult::success(vec![ContentBlock::text(ok.content_text)]),
        Err(err) => CallToolResult::error(vec![ContentBlock::text(err.content_text)]),
    }
}

fn json_query_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    }
}

fn build_get_path(api_path: &str, args: &Value) -> String {
    let Value::Object(map) = args else {
        return api_path.to_string();
    };
    let mut pairs: Vec<(String, String)> = Vec::new();
    for (key, value) in map {
        if let Some(v) = json_query_value(value) {
            pairs.push((key.clone(), v));
        }
    }
    if pairs.is_empty() {
        return api_path.to_string();
    }
    let query = pairs
        .into_iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{api_path}?{query}")
}

/// Proxy one allowlisted tool call to Sidecar HTTP loopback and map the response.
///
/// Outer `Err` = protocol/routing failure (unknown slot/tool).
/// Inner `Result` = Node-equivalent tool success vs tool error.
pub async fn proxy_tool_call(
    sidecar_base_url: &str,
    slot: &str,
    tool: &str,
    args: Value,
) -> Result<Result<McpToolResult, McpToolError>, String> {
    let table = build_slot_tool_table(slot)
        .ok_or_else(|| format!("unregistered scene_slot: {slot}"))?;
    let route = table
        .tools
        .iter()
        .find(|r| r.name == tool)
        .ok_or_else(|| format!("tool '{tool}' is not allowlisted for slot '{slot}'"))?;

    let base = sidecar_base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let response = match route.method {
        HttpMethod::Get => {
            let path = build_get_path(&route.api_path, &args);
            let url = format!("{base}{path}");
            client.get(url).send().await
        }
        HttpMethod::Post => {
            let url = format!("{base}{}", route.api_path);
            client.post(url).json(&args).send().await
        }
    };

    let response = match response {
        Ok(res) => res,
        Err(err) => {
            return Ok(Err(McpToolError {
                content_text: format!("HTTP 503: Workbench HTTP unreachable: {err}"),
            }));
        }
    };

    let status = response.status().as_u16();
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("read sidecar body: {e}"))?;
    Ok(map_sidecar_response_to_mcp(status, &body))
}

/// Sidecar HTTP method for a tool→`/api/*` route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

/// One allowlisted tool and its Sidecar outbound path (from Node `registerTool`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRoute {
    pub name: String,
    pub method: HttpMethod,
    pub api_path: String,
}

/// Authoritative slot→tool routing table for a registered `scene_slot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotToolTable {
    pub scene_slot: String,
    pub include_corpus: bool,
    pub include_todo: bool,
    pub tools: Vec<ToolRoute>,
}

/// Observable tool descriptor for `tools/list` contract checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub name: String,
}

/// Registered scene_slot → API surface seeds (ported from Node `SCENE_SLOT_API`).
#[derive(Debug, Clone, Copy)]
struct SceneSlotApi {
    include_corpus: bool,
    include_todo: bool,
}

const REGISTERED_SCENE_SLOTS: &[&str] = &["todo_task", "cursor_ide"];

fn scene_slot_api(slot: &str) -> Option<SceneSlotApi> {
    match slot {
        "todo_task" => Some(SceneSlotApi {
            include_corpus: false,
            include_todo: true,
        }),
        "cursor_ide" => Some(SceneSlotApi {
            include_corpus: true,
            include_todo: true,
        }),
        _ => None,
    }
}

fn corpus_tool_routes() -> Vec<ToolRoute> {
    vec![
        ToolRoute {
            name: "get_corpus_catalog".into(),
            method: HttpMethod::Get,
            api_path: "/api/corpus-catalog".into(),
        },
        ToolRoute {
            name: "get_corpus_files".into(),
            method: HttpMethod::Post,
            api_path: "/api/corpus-files".into(),
        },
        ToolRoute {
            name: "archive_document".into(),
            method: HttpMethod::Post,
            api_path: "/api/archive-document".into(),
        },
        ToolRoute {
            name: "archive_digest".into(),
            method: HttpMethod::Post,
            api_path: "/api/archive-digest".into(),
        },
    ]
}

fn todo_tool_routes() -> Vec<ToolRoute> {
    vec![
        ToolRoute {
            name: "create_todo_task".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-create".into(),
        },
        ToolRoute {
            name: "update_todo_task".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-update".into(),
        },
        ToolRoute {
            name: "list_todo_tasks".into(),
            method: HttpMethod::Get,
            api_path: "/api/todo-tasks".into(),
        },
        ToolRoute {
            name: "list_todo_categories".into(),
            method: HttpMethod::Get,
            api_path: "/api/todo-task-list-categories".into(),
        },
        ToolRoute {
            name: "get_todo_task".into(),
            method: HttpMethod::Get,
            api_path: "/api/todo-task".into(),
        },
        ToolRoute {
            name: "delete_todo_task".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-delete".into(),
        },
        ToolRoute {
            name: "add_todo_sub".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-add-sub".into(),
        },
        ToolRoute {
            name: "update_todo_sub".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-update-sub".into(),
        },
        ToolRoute {
            name: "delete_todo_sub".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-delete-sub".into(),
        },
        ToolRoute {
            name: "complete_todo".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-complete".into(),
        },
        ToolRoute {
            name: "link_todo_archive".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-link-archive".into(),
        },
        ToolRoute {
            name: "add_todo_attachment".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-add-attachment".into(),
        },
        ToolRoute {
            name: "list_todo_attachments".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-list-attachments".into(),
        },
        ToolRoute {
            name: "get_todo_attachment".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-get-attachment".into(),
        },
        ToolRoute {
            name: "update_todo_attachment".into(),
            method: HttpMethod::Post,
            api_path: "/api/todo-task-update-attachment".into(),
        },
    ]
}

/// Build the Host-authoritative slot→tool routing table.
/// Unregistered slots return `None` (caller must HTTP hard-reject).
pub fn build_slot_tool_table(slot: &str) -> Option<SlotToolTable> {
    let api = scene_slot_api(slot)?;
    let mut tools = Vec::new();
    if api.include_corpus {
        tools.extend(corpus_tool_routes());
    }
    if api.include_todo {
        tools.extend(todo_tool_routes());
    }
    Some(SlotToolTable {
        scene_slot: slot.to_string(),
        include_corpus: api.include_corpus,
        include_todo: api.include_todo,
        tools,
    })
}

/// Allowlisted tool names for a slot (empty when unregistered).
pub fn tools_list_for_slot(slot: &str) -> Vec<ToolDescriptor> {
    build_slot_tool_table(slot)
        .map(|table| {
            table
                .tools
                .into_iter()
                .map(|t| ToolDescriptor { name: t.name })
                .collect()
        })
        .unwrap_or_default()
}

/// HTTP hard-reject payload for an unregistered `scene_slot` (no MCP session).
pub fn reject_unknown_slot(scene_slot: &str) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    let body = serde_json::json!({
        "error": "unknown_scene_slot",
        "scene_slot": scene_slot,
        "message": "Unregistered scene_slot; connection rejected",
    });
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}

fn reject_scene_slot_required() -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
    let body = serde_json::json!({
        "error": "scene_slot_required",
        "message": format!(
            "Use /mcp/<scene_slot> (registered: {})",
            REGISTERED_SCENE_SLOTS.join(", ")
        ),
    });
    (
        StatusCode::NOT_FOUND,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
}

/// Configuration for the embedded Host MCP runtime.
#[derive(Debug, Clone)]
pub struct McpRuntimeConfig {
    pub bind_addr: SocketAddr,
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

/// Per-slot MCP handler; proxies allowlisted tools to Sidecar HTTP loopback.
#[derive(Clone)]
struct SlotHandler {
    scene_slot: String,
    sidecar_base_url: String,
}

fn empty_object_schema() -> Arc<serde_json::Map<String, serde_json::Value>> {
    let mut schema = serde_json::Map::new();
    schema.insert("type".into(), serde_json::Value::String("object".into()));
    Arc::new(schema)
}

impl ServerHandler for SlotHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        // Names come from the Host routing table; full input schemas land in later tasks.
        let tools: Vec<Tool> = tools_list_for_slot(&self.scene_slot)
            .into_iter()
            .map(|d| Tool::new(d.name, "", empty_object_schema()))
            .collect();
        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + Send + '_ {
        let slot = self.scene_slot.clone();
        let base = self.sidecar_base_url.clone();
        async move {
            let args = Value::Object(request.arguments.unwrap_or_default());
            match proxy_tool_call(&base, &slot, request.name.as_ref(), args).await {
                Ok(mapped) => Ok(mapped_to_call_tool_result(mapped).into()),
                Err(msg) => Err(McpError::invalid_params(msg, None)),
            }
        }
    }
}

/// Readiness probe contract matching Node `GET /health` (`ok` + non-empty `mcp`).
fn health_json(port: u16) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
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

async fn bare_mcp_reject() -> impl IntoResponse {
    reject_scene_slot_required()
}

async fn unknown_scene_slot_reject(Path(scene_slot): Path<String>) -> impl IntoResponse {
    reject_unknown_slot(&scene_slot)
}

fn mount_slot_service(
    router: Router,
    scene_slot: &'static str,
    cancel: CancellationToken,
    sidecar_base_url: String,
) -> Router {
    let slot = scene_slot.to_string();
    let service: StreamableHttpService<SlotHandler, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                Ok(SlotHandler {
                    scene_slot: slot.clone(),
                    sidecar_base_url: sidecar_base_url.clone(),
                })
            },
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default().with_cancellation_token(cancel),
        );
    router.nest_service(&format!("/mcp/{scene_slot}"), service)
}

fn build_router(port: u16, cancel: CancellationToken, sidecar_base_url: String) -> Router {
    let mut router = Router::new()
        .route("/health", get(move || async move { health_json(port) }))
        .route("/mcp", any(bare_mcp_reject));

    // Registered slots first so StreamableHttpService owns those paths exclusively.
    for slot in REGISTERED_SCENE_SLOTS {
        router = mount_slot_service(router, slot, cancel.clone(), sidecar_base_url.clone());
    }

    // Unregistered `/mcp/<scene_slot>`: HTTP 404 JSON only — no LocalSessionManager / MCP session.
    router.route("/mcp/{scene_slot}", any(unknown_scene_slot_reject))
}

/// Start MCP on an independent OS thread with its own tokio runtime.
pub fn start_embedded_mcp_runtime(
    config: McpRuntimeConfig,
) -> Result<McpRuntimeHandle, McpStartError> {
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
                    let _ = tx.send(Err(format!(
                        "cannot bind {bind_addr}: {err}"
                    )));
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

            let port = local_addr.port();
            let router = build_router(
                port,
                cancel_child.clone(),
                DEFAULT_SIDECAR_BASE_URL.to_string(),
            );
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

/// Bind `bind_addr` with registered-slot Streamable HTTP MCP routes under `/mcp/<scene_slot>`.
pub fn start_mcp_listener(bind_addr: SocketAddr) -> Result<McpRuntimeHandle, McpStartError> {
    start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr })
}

#[cfg(test)]
#[path = "../unit-tests/services/mcp_protocol_adapter_tests.rs"]
mod tests;
