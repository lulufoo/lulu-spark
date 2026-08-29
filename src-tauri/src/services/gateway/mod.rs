//! In-process LAN HTTPS Gateway: named routes only, no ticket issuance.

use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use axum::body::to_bytes;
use axum::extract::{Request, State};
use axum::http::{header, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::Router;
use axum_server::tls_rustls::RustlsConfig;
use axum_server::Handle;
use base64::Engine;
use sha2::{Digest, Sha256};

use crate::config::settings::AppSettings;
use crate::services::lan_ip::current_lan_ipv4;

pub const GATEWAY_CERT_FILE: &str = "gateway-cert.pem";
pub const GATEWAY_KEY_FILE: &str = "gateway-key.pem";

#[derive(Debug)]
pub enum GatewayError {
    BindFailed(String),
    Tls(String),
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindFailed(msg) => write!(f, "bind failed: {msg}"),
            Self::Tls(msg) => write!(f, "tls: {msg}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GatewayConfig {
    pub listen_port: u16,
    pub mcp_port: u16,
    pub sidecar_port: u16,
    pub config_dir: PathBuf,
}

pub struct GatewayHandle {
    local_addr: SocketAddr,
    tls_fingerprint: String,
    shutdown: Handle,
    join: Option<JoinHandle<()>>,
}

impl GatewayHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub fn tls_fingerprint(&self) -> &str {
        &self.tls_fingerprint
    }
}

pub enum BootDecision {
    SkippedNoLanIp,
    Started(GatewayHandle),
    Failed(GatewayError),
}

impl std::fmt::Debug for BootDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SkippedNoLanIp => write!(f, "SkippedNoLanIp"),
            Self::Started(_) => write!(f, "Started(..)"),
            Self::Failed(err) => write!(f, "Failed({err})"),
        }
    }
}

pub struct GatewayState {
    handle: Mutex<Option<GatewayHandle>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayListen {
    pub port: u16,
    pub tls_fingerprint: String,
}

impl GatewayState {
    pub fn new() -> Self {
        Self {
            handle: Mutex::new(None),
        }
    }

    pub fn set(&self, handle: GatewayHandle) {
        if let Ok(mut guard) = self.handle.lock() {
            *guard = Some(handle);
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                stop(handle);
            }
        }
    }

    pub fn current(&self) -> Option<GatewayListen> {
        let guard = self.handle.lock().ok()?;
        guard.as_ref().map(|handle| GatewayListen {
            port: handle.local_addr().port(),
            tls_fingerprint: handle.tls_fingerprint().to_string(),
        })
    }
}

#[derive(Clone)]
struct GwState {
    mcp_port: u16,
    sidecar_port: u16,
}

pub fn advertised_address(port: u16) -> Option<String> {
    current_lan_ipv4().map(|ip| format!("{ip}:{port}"))
}

pub fn boot(settings: &AppSettings, config_dir: PathBuf) -> BootDecision {
    boot_with(GatewayConfig {
        listen_port: settings.effective_gateway_port(),
        mcp_port: settings.effective_mcp_port(),
        sidecar_port: settings.effective_http_port(),
        config_dir,
    })
}

pub fn boot_with(config: GatewayConfig) -> BootDecision {
    if current_lan_ipv4().is_none() {
        return BootDecision::SkippedNoLanIp;
    }
    match start(config) {
        Ok(handle) => BootDecision::Started(handle),
        Err(err) => BootDecision::Failed(err),
    }
}

fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub fn start(config: GatewayConfig) -> Result<GatewayHandle, GatewayError> {
    install_crypto_provider();
    let tls_fingerprint = ensure_tls_certificate(&config.config_dir)?;
    let cert_path = config.config_dir.join(GATEWAY_CERT_FILE);
    let key_path = config.config_dir.join(GATEWAY_KEY_FILE);
    let tcp = std::net::TcpListener::bind(SocketAddr::from((
        Ipv4Addr::UNSPECIFIED,
        config.listen_port,
    )))
    .map_err(|err| GatewayError::BindFailed(err.to_string()))?;
    tcp.set_nonblocking(true)
        .map_err(|err| GatewayError::BindFailed(err.to_string()))?;
    let local_addr = tcp
        .local_addr()
        .map_err(|err| GatewayError::BindFailed(err.to_string()))?;

    let shutdown = Handle::new();
    let shutdown_thread = shutdown.clone();
    let mcp_port = config.mcp_port;
    let sidecar_port = config.sidecar_port;
    let join = thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        rt.block_on(async move {
            install_crypto_provider();
            let Ok(tls) = RustlsConfig::from_pem_file(cert_path, key_path).await else {
                return;
            };
            let app = build_router(mcp_port, sidecar_port);
            let _ = axum_server::from_tcp_rustls(tcp, tls)
                .handle(shutdown_thread)
                .serve(app.into_make_service())
                .await;
        });
    });

    wait_until_accepting(local_addr.port())?;
    Ok(GatewayHandle {
        local_addr,
        tls_fingerprint,
        shutdown,
        join: Some(join),
    })
}

pub fn stop(mut handle: GatewayHandle) {
    handle.shutdown.shutdown();
    if let Some(join) = handle.join.take() {
        let _ = join.join();
    }
}

pub fn ensure_tls_certificate(config_dir: &Path) -> Result<String, GatewayError> {
    std::fs::create_dir_all(config_dir).map_err(|err| GatewayError::Tls(err.to_string()))?;
    let cert_path = config_dir.join(GATEWAY_CERT_FILE);
    let key_path = config_dir.join(GATEWAY_KEY_FILE);
    if !(cert_path.is_file() && key_path.is_file()) {
        let certified = rcgen::generate_simple_self_signed(vec![
            "localhost".to_string(),
            "lulu-workbench-gateway".to_string(),
        ])
        .map_err(|err| GatewayError::Tls(err.to_string()))?;
        std::fs::write(&cert_path, certified.cert.pem())
            .map_err(|err| GatewayError::Tls(err.to_string()))?;
        std::fs::write(&key_path, certified.key_pair.serialize_pem())
            .map_err(|err| GatewayError::Tls(err.to_string()))?;
    }
    let pem = std::fs::read_to_string(&cert_path).map_err(|err| GatewayError::Tls(err.to_string()))?;
    Ok(sha256_hex(&pem_to_der(&pem)?))
}

fn pem_to_der(pem: &str) -> Result<Vec<u8>, GatewayError> {
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .map_err(|err| GatewayError::Tls(err.to_string()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn wait_until_accepting(port: u16) -> Result<(), GatewayError> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(50)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(GatewayError::BindFailed(format!(
        "gateway did not accept on 127.0.0.1:{port}"
    )))
}

fn build_router(mcp_port: u16, sidecar_port: u16) -> Router {
    Router::new()
        .route("/bind/complete", any(bind_complete_named))
        .route("/mcp/mobile", any(forward_named))
        .route("/health", get(forward_named))
        .fallback(reject_unnamed)
        .with_state(GwState {
            mcp_port,
            sidecar_port,
        })
}

async fn bind_complete_named(State(state): State<GwState>, request: Request) -> Response {
    if request.method() != Method::POST {
        return reject_unnamed().await;
    }
    let headers = hop_headers(request.headers(), BIND_FORWARD_HEADERS);
    let body = match to_bytes(request.into_body(), 2 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => Vec::new(),
    };
    let sidecar_port = state.sidecar_port;
    match tokio::task::spawn_blocking(move || {
        proxy_loopback(
            sidecar_port,
            Method::POST,
            "/api/bind-complete".to_string(),
            headers,
            body,
        )
    })
    .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(msg)) => json_status(
            StatusCode::BAD_GATEWAY,
            &serde_json::json!({ "error": msg }).to_string(),
        ),
        Err(_) => json_status(StatusCode::BAD_GATEWAY, r#"{"error":"forward_join"}"#),
    }
}

async fn forward_named(State(state): State<GwState>, request: Request) -> Response {
    let method = request.method().clone();
    let path = request
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());
    let headers = hop_headers(request.headers(), MCP_FORWARD_HEADERS);
    let body = match to_bytes(request.into_body(), 2 * 1024 * 1024).await {
        Ok(bytes) => bytes.to_vec(),
        Err(_) => Vec::new(),
    };
    let mcp_port = state.mcp_port;
    match tokio::task::spawn_blocking(move || {
        proxy_loopback(mcp_port, method, path, headers, body)
    })
    .await
    {
        Ok(Ok(response)) => response,
        Ok(Err(msg)) => json_status(
            StatusCode::BAD_GATEWAY,
            &serde_json::json!({ "error": msg }).to_string(),
        ),
        Err(_) => json_status(StatusCode::BAD_GATEWAY, r#"{"error":"forward_join"}"#),
    }
}

fn hop_headers(
    incoming: &axum::http::HeaderMap,
    names: &[&str],
) -> Vec<(String, String)> {
    names
        .iter()
        .filter_map(|name| {
            incoming
                .get(*name)
                .and_then(|value| value.to_str().ok())
                .map(|value| ((*name).to_string(), value.to_string()))
        })
        .collect()
}

fn proxy_loopback(
    mcp_port: u16,
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
) -> Result<Response, String> {
    let url = format!("http://127.0.0.1:{mcp_port}{path}");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|err| err.to_string())?;
    let reqwest_method = reqwest::Method::from_bytes(method.as_str().as_bytes())
        .map_err(|err| err.to_string())?;
    let mut request = client.request(reqwest_method, url).body(body);
    for (name, value) in headers {
        request = request.header(name, value);
    }
    let response = request.send().map_err(|err| err.to_string())?;
    let status = StatusCode::from_u16(response.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let session = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string());
    let payload = response.text().map_err(|err| err.to_string())?;
    let mut builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type);
    if let Some(session) = session {
        builder = builder.header("mcp-session-id", session);
    }
    builder.body(payload.into()).map_err(|err| err.to_string())
}

const BIND_FORWARD_HEADERS: &[&str] = &["authorization", "content-type"];
const MCP_FORWARD_HEADERS: &[&str] = &[
    "authorization",
    "content-type",
    "accept",
    "mcp-session-id",
    "mcp-protocol-version",
];

async fn reject_unnamed() -> Response {
    json_status(StatusCode::NOT_FOUND, r#"{"error":"not_found"}"#)
}

fn json_status(status: StatusCode, body: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}

#[cfg(test)]
#[path = "../../unit-tests/services/gateway.rs"]
mod tests;
