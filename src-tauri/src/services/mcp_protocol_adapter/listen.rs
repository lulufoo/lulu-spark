//! Streamable HTTP listen, bearer gates, and close-gate smoke.

use std::net::SocketAddr;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use rmcp::{
    ServerHandler, ServiceExt,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ClientCapabilities, ClientInfo,
        ContentBlock, Implementation, ListToolsResult, PaginatedRequestParams, ServerCapabilities,
        ServerInfo, Tool, ToolAnnotations,
    },
    service::RequestContext,
    transport::{
        StreamableHttpClientTransport,
        streamable_http_server::{
            session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
        },
    },
    ErrorData as McpError, RoleServer,
};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::services::local_http;
use crate::services::mcp_oauth::{verify_device_token, verify_for_slot, OAuthError, Slot, TicketHandle};
use super::proxy::{mapped_to_call_tool_result, proxy_tool_call};
use super::types::ToolRoute;
use super::channel_routes::build_channel_tool_table;
use super::routes::scene_slot_api;
use super::types::{
    CloseGateError, CloseGateReport, McpRuntimeConfig, McpRuntimeHandle, McpStartError,
    McpStopError, McpToolError, DEFAULT_SIDECAR_BASE_URL, REGISTERED_SCENE_SLOTS,
};

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

pub(super) fn reject_scene_slot_required() -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
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
pub(super) struct SlotHandler {
    scene_slot: String,
    channel: String,
    sidecar_base_url: String,
}

pub(super) fn route_to_mcp_tool(route: ToolRoute) -> Tool {
    let schema = route
        .input_schema
        .as_object()
        .cloned()
        .expect("MCP route input schema must be a JSON object");
    let annotations = ToolAnnotations::new()
        .read_only(route.read_only)
        .destructive(route.destructive)
        .idempotent(route.read_only)
        .open_world(false);
    Tool::new(route.name, route.description, Arc::new(schema)).with_annotations(annotations)
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
        let enabled = crate::services::mcp_channel_tools::enabled_names(&self.channel);
        let tools: Vec<Tool> = build_channel_tool_table(&self.scene_slot, &self.channel)
            .map(|table| {
                table
                    .tools
                    .into_iter()
                    .filter(|route| enabled.contains(&route.name))
                    .map(route_to_mcp_tool)
                    .collect()
            })
            .unwrap_or_default();
        std::future::ready(Ok(ListToolsResult::with_all_items(tools)))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>> + Send + '_ {
        let slot = self.scene_slot.clone();
        let channel = self.channel.clone();
        let base = self.sidecar_base_url.clone();
        async move {
            if !crate::services::mcp_channel_tools::is_enabled(&channel, request.name.as_ref()) {
                return Err(McpError::invalid_params(
                    format!(
                        "tool '{}' is not enabled for channel '{}'",
                        request.name, channel
                    ),
                    None,
                ));
            }
            let args = Value::Object(request.arguments.unwrap_or_default());
            match proxy_tool_call(&base, &slot, &channel, request.name.as_ref(), args).await {
                Ok(mapped) => Ok(mapped_to_call_tool_result(mapped).into()),
                Err(msg) => Err(McpError::invalid_params(msg, None)),
            }
        }
    }
}

/// Readiness probe contract matching Node `GET /health` (`ok` + non-empty `mcp`).
pub(super) fn health_json(port: u16) -> (StatusCode, [(header::HeaderName, &'static str); 1], String) {
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

pub(super) async fn bare_mcp_reject() -> impl IntoResponse {
    reject_scene_slot_required()
}

pub(super) async fn unknown_scene_slot_reject(Path(scene_slot): Path<String>) -> impl IntoResponse {
    reject_unknown_slot(&scene_slot)
}

pub(super) fn mount_mobile_service(
    router: Router,
    cancel: CancellationToken,
    sidecar_base_url: String,
) -> Router {
    let service: StreamableHttpService<SlotHandler, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                Ok(SlotHandler {
                    scene_slot: "workbench".to_string(),
                    channel: "mobile".to_string(),
                    sidecar_base_url: sidecar_base_url.clone(),
                })
            },
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default().with_cancellation_token(cancel),
        );
    let gated = Router::new()
        .fallback_service(service)
        .layer(middleware::from_fn(mobile_device_bearer_gate));
    router.nest_service("/mcp/mobile", gated)
}

pub(super) fn mount_slot_service(
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
                    channel: slot.clone(),
                    sidecar_base_url: sidecar_base_url.clone(),
                })
            },
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default().with_cancellation_token(cancel),
        );
    let gated = Router::new()
        .fallback_service(service)
        .layer(middleware::from_fn_with_state(scene_slot, slot_bearer_gate));
    router.nest_service(&format!("/mcp/{scene_slot}"), gated)
}

pub(super) fn build_router(port: u16, cancel: CancellationToken, sidecar_base_url: String) -> Router {
    let mut router = Router::new()
        .route("/health", get(move || async move { health_json(port) }))
        .route("/mcp", any(bare_mcp_reject));

    // Registered slots first so StreamableHttpService owns those paths exclusively.
    for slot in REGISTERED_SCENE_SLOTS {
        router = mount_slot_service(router, slot, cancel.clone(), sidecar_base_url.clone());
    }

    // Independent /mcp/mobile nest — not a registered slot; must precede catch-all 404.
    router = mount_mobile_service(router, cancel, sidecar_base_url);

    // Unregistered `/mcp/<scene_slot>`: HTTP 404 JSON only — no LocalSessionManager / MCP session.
    router.route("/mcp/{scene_slot}", any(unknown_scene_slot_reject))
}

pub(super) fn bearer_secret(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|secret| !secret.is_empty())
}

pub(super) fn verify_registered_slot(scene_slot: &str, headers: &HeaderMap) -> Result<(), StatusCode> {
    let slot = Slot::parse(scene_slot).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let Some(secret) = bearer_secret(headers) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    match verify_for_slot(slot, TicketHandle::from_secret(secret)) {
        Ok(()) => Ok(()),
        Err(OAuthError::rejected) | Err(OAuthError::keychain_unavailable) => {
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(OAuthError::slot_unknown) => Err(StatusCode::UNAUTHORIZED),
    }
}

pub(super) async fn slot_bearer_gate(
    State(scene_slot): State<&'static str>,
    request: Request,
    next: Next,
) -> Response {
    if let Err(status) = verify_registered_slot(scene_slot, request.headers()) {
        return status.into_response();
    }
    next.run(request).await
}

pub(super) fn verify_mobile_device(headers: &HeaderMap) -> Result<(), StatusCode> {
    let Some(secret) = bearer_secret(headers) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    match verify_device_token(secret) {
        Ok(_) => Ok(()),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

pub(super) async fn mobile_device_bearer_gate(request: Request, next: Next) -> Response {
    if let Err(status) = verify_mobile_device(request.headers()) {
        return status.into_response();
    }
    next.run(request).await
}

pub(super) fn fetch_json_ok(
    client: &reqwest::blocking::Client,
    url: &str,
    label: &str,
) -> Result<Value, CloseGateError> {
    let response = client.get(url).send().map_err(|e| {
        CloseGateError::DualListenNotObservable(format!("{label}: {e}"))
    })?;
    if !response.status().is_success() {
        return Err(CloseGateError::DualListenNotObservable(format!(
            "{label} status {}",
            response.status()
        )));
    }
    let json: Value = response.json().map_err(|e| {
        CloseGateError::DualListenNotObservable(format!("{label} JSON: {e}"))
    })?;
    if json.get("ok") != Some(&Value::Bool(true)) {
        return Err(CloseGateError::DualListenNotObservable(format!(
            "{label} not ok: {json}"
        )));
    }
    Ok(json)
}

/// Observe same-process Sidecar HTTP (`/api/status`) + Host MCP (`/health`) dual listen.
pub fn observe_dual_listen(mcp_port: u16, sidecar_port: u16) -> Result<bool, CloseGateError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| CloseGateError::DualListenNotObservable(format!("http client: {e}")))?;

    let _sidecar = fetch_json_ok(
        &client,
        &format!("http://127.0.0.1:{sidecar_port}/api/status"),
        &format!("sidecar :{sidecar_port}"),
    )?;
    let health_json = fetch_json_ok(
        &client,
        &format!("http://127.0.0.1:{mcp_port}/health"),
        &format!("mcp :{mcp_port} /health"),
    )?;
    let mcp_field = health_json
        .get("mcp")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if mcp_field.trim().is_empty() {
        return Err(CloseGateError::DualListenNotObservable(format!(
            "mcp /health mcp field empty: {health_json}"
        )));
    }

    Ok(true)
}

pub(super) async fn run_initialize_and_list_tools(
    mcp_port: u16,
    slot: &str,
) -> Result<Vec<String>, CloseGateError> {
    let url = format!("http://127.0.0.1:{mcp_port}/mcp/{slot}");
    let transport = StreamableHttpClientTransport::from_uri(url);
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("host-close-gate", "0.1.0"),
    );
    let client = client_info
        .serve(transport)
        .await
        .map_err(|e| CloseGateError::InitializeFailed(format!("{e:#}")))?;

    let tools = client
        .list_tools(Default::default())
        .await
        .map_err(|e| CloseGateError::ToolsListFailed(format!("{e:#}")))?;
    let names: Vec<String> = tools
        .tools
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    let _ = client.cancel().await;
    Ok(names)
}

/// P1/V5 close gate: observe `:9876`+`:8765` and complete initialize + tools/list on a registered slot.
///
/// Must pass before removing Node spawn paths (P2/T7).
pub fn close_gate_smoke_initialize_list(slot: &str) -> Result<CloseGateReport, CloseGateError> {
    if scene_slot_api(slot).is_none() {
        return Err(CloseGateError::UnregisteredSlot(slot.to_string()));
    }

    let mcp_port = crate::DEFAULT_MCP_PORT;
    let sidecar_port = local_http::DEFAULT_HTTP_PORT;
    let dual_listen_observed = observe_dual_listen(mcp_port, sidecar_port)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CloseGateError::Runtime(format!("tokio runtime: {e}")))?;
    let tool_names = rt.block_on(run_initialize_and_list_tools(mcp_port, slot))?;

    Ok(CloseGateReport {
        mcp_port,
        sidecar_port,
        scene_slot: slot.to_string(),
        dual_listen_observed,
        initialize_ok: true,
        tools_list_ok: true,
        tool_names,
    })
}

/// Start MCP on an independent OS thread with its own tokio runtime.
pub fn start_embedded_mcp_runtime(
    config: McpRuntimeConfig,
) -> Result<McpRuntimeHandle, McpStartError> {
    start_embedded_mcp_runtime_with_sidecar(config, DEFAULT_SIDECAR_BASE_URL.to_string())
}

/// Start MCP with an explicit Sidecar HTTP base. Production uses
/// [`start_embedded_mcp_runtime`]; this entry keeps integration tests isolated
/// on ephemeral loopback ports.
pub fn start_embedded_mcp_runtime_with_sidecar(
    config: McpRuntimeConfig,
    sidecar_base_url: String,
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
                sidecar_base_url,
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
