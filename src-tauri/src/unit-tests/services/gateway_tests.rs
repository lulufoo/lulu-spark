use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine;
use sha2::{Digest, Sha256};

use super::*;
use crate::config::settings::{
    DEFAULT_PROD_GATEWAY_PORT, DEFAULT_PROD_HTTP_PORT, DEFAULT_PROD_MCP_PORT,
    DEFAULT_SANDBOX_GATEWAY_PORT, DEFAULT_SANDBOX_HTTP_PORT, DEFAULT_SANDBOX_MCP_PORT,
};
use crate::services::bind::{
    complete_bind, create_bind_payload, seal_bind_request, test_clear_session,
    test_reset_bind_keychain,
};
use crate::services::lan_ip::{test_override_nics, NicIpv4};
use crate::services::local_http;
use crate::test_support::TestSandbox;

fn nic(name: &str, addr: &str) -> NicIpv4 {
    NicIpv4 {
        name: name.to_string(),
        addr: addr.parse().expect("ipv4"),
        up: true,
    }
}

fn with_nics(nics: Option<Vec<NicIpv4>>, test: impl FnOnce()) {
    test_override_nics(nics);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
    test_override_nics(None);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

fn ephemeral_loopback_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

fn https_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("https client")
}

fn gw_url(handle: &GatewayHandle, path: &str) -> String {
    format!("https://127.0.0.1:{}{path}", handle.local_addr().port())
}

fn start_gw(mcp_port: u16, config_dir: &Path) -> GatewayHandle {
    start_gw_with_sidecar(mcp_port, 9, config_dir)
}

fn start_gw_with_sidecar(mcp_port: u16, sidecar_port: u16, config_dir: &Path) -> GatewayHandle {
    start(GatewayConfig {
        listen_port: 0,
        mcp_port,
        sidecar_port,
        config_dir: config_dir.to_path_buf(),
    })
    .expect("start gateway")
}

fn pem_to_der(pem: &str) -> Vec<u8> {
    let body: String = pem
        .lines()
        .filter(|line| !line.starts_with("-----"))
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(body)
        .expect("cert pem body")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

struct MockMcp {
    port: u16,
    hits: Arc<Mutex<Vec<(String, String, Vec<u8>)>>>,
    server: Arc<tiny_http::Server>,
    join: Option<thread::JoinHandle<()>>,
}

impl MockMcp {
    fn start() -> Self {
        let port = ephemeral_loopback_port();
        let server = Arc::new(
            tiny_http::Server::http(format!("127.0.0.1:{port}")).expect("mock mcp listen"),
        );
        let hits = Arc::new(Mutex::new(Vec::new()));
        let server_thread = Arc::clone(&server);
        let hits_thread = Arc::clone(&hits);
        let join = thread::spawn(move || {
            for mut request in server_thread.incoming_requests() {
                let method = request.method().to_string();
                let path = request.url().split('?').next().unwrap_or("").to_string();
                let mut body = Vec::new();
                let _ = request.as_reader().read_to_end(&mut body);
                hits_thread
                    .lock()
                    .expect("hits")
                    .push((method.clone(), path.clone(), body.clone()));
                let (status, payload) = if method == "GET" && path == "/health" {
                    (
                        200,
                        serde_json::json!({
                            "ok": true,
                            "mcp": format!("http://127.0.0.1:{port}/mcp/<scene_slot>")
                        })
                        .to_string(),
                    )
                } else if path == "/mcp/mobile" {
                    (
                        200,
                        serde_json::json!({
                            "forwarded": true,
                            "path": path,
                            "echo": String::from_utf8_lossy(&body),
                        })
                        .to_string(),
                    )
                } else {
                    (599, serde_json::json!({ "leaked": path }).to_string())
                };
                let response = tiny_http::Response::from_string(payload)
                    .with_status_code(tiny_http::StatusCode(status))
                    .with_header(
                        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                            .expect("header"),
                    );
                let _ = request.respond(response);
            }
        });
        Self {
            port,
            hits,
            server,
            join: Some(join),
        }
    }

    fn hits(&self) -> Vec<(String, String, Vec<u8>)> {
        self.hits.lock().expect("hits").clone()
    }
}

impl Drop for MockMcp {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn with_bind_sandbox(test: impl FnOnce(&Path)) {
    let sandbox = TestSandbox::new();
    test_reset_bind_keychain();
    test_clear_session();
    let config_dir = sandbox
        .config_file_path()
        .parent()
        .expect("config dir")
        .to_path_buf();
    test(&config_dir);
    test_clear_session();
}

#[test]
fn gateway_listens_on_unspecified_and_advertises_lan_ip_port() {
    with_nics(
        Some(vec![nic("en0", "192.168.1.8")]),
        || {
            let dir = tempfile::tempdir().expect("tmp");
            let mock = MockMcp::start();
            let handle = start_gw(mock.port, dir.path());
            assert_eq!(
                handle.local_addr().ip(),
                Ipv4Addr::UNSPECIFIED,
                "socket must listen 0.0.0.0"
            );
            assert_eq!(
                advertised_address(DEFAULT_PROD_GATEWAY_PORT).as_deref(),
                Some("192.168.1.8:7654")
            );
            assert_eq!(
                advertised_address(DEFAULT_SANDBOX_GATEWAY_PORT).as_deref(),
                Some("192.168.1.8:17654")
            );
            stop(handle);
        },
    );
}

#[test]
fn boot_skips_gateway_when_lan_ip_is_missing() {
    with_nics(Some(vec![nic("lo0", "127.0.0.1")]), || {
        let dir = tempfile::tempdir().expect("tmp");
        let occupied = ephemeral_loopback_port();
        match boot_with(GatewayConfig {
            listen_port: occupied,
            mcp_port: occupied,
            sidecar_port: occupied,
            config_dir: dir.path().to_path_buf(),
        }) {
            BootDecision::SkippedNoLanIp => {}
            other => panic!("expected skip when current_lan_ipv4 is empty, got {other:?}"),
        }
        let bound = TcpListener::bind(format!("0.0.0.0:{occupied}"));
        assert!(
            bound.is_ok(),
            "skipped boot must not occupy the gateway port"
        );
    });
}

#[test]
fn post_bind_complete_forwards_ciphertext_to_sidecar() {
    with_bind_sandbox(|config_dir| {
        with_nics(Some(vec![nic("en0", "10.0.0.4")]), || {
            let mock = MockMcp::start();
            let sidecar_port = ephemeral_loopback_port();
            let sidecar = local_http::start(config_dir.to_path_buf(), sidecar_port)
                .expect("start sidecar");
            let handle = start_gw_with_sidecar(mock.port, sidecar_port, config_dir);
            let payload =
                create_bind_payload(Ipv4Addr::new(10, 0, 0, 4), 17654, handle.tls_fingerprint())
                    .expect("draw");
            let sealed =
                seal_bind_request(&payload.temp_pub, "phone-gw", Some("Pixel")).expect("seal");
            let response = https_client()
                .post(gw_url(&handle, "/bind/complete"))
                .header("content-type", "application/octet-stream")
                .body(sealed.clone())
                .send()
                .expect("bind complete");
            assert_eq!(response.status().as_u16(), 200);
            let body: serde_json::Value = response.json().expect("json");
            assert_eq!(
                body["device_mcp_token"].as_str().map(|s| s.len()),
                Some(64)
            );
            assert_eq!(
                body["binding_public_key"].as_str().map(|s| s.len()),
                Some(64)
            );
            assert!(
                mock.hits().is_empty(),
                "bind complete must not hit MCP"
            );
            assert_eq!(
                complete_bind(&sealed).expect_err("session consumed via sidecar"),
                crate::services::bind::BindError::consumed
            );
            stop(handle);
            local_http::stop(sidecar);
        });
    });
}

#[test]
fn named_forward_mcp_mobile_to_loopback_mcp_port() {
    let dir = tempfile::tempdir().expect("tmp");
    let mock = MockMcp::start();
    let handle = start_gw(mock.port, dir.path());
    let response = https_client()
        .post(gw_url(&handle, "/mcp/mobile"))
        .header("authorization", "Bearer device-ticket")
        .header("content-type", "application/json")
        .body(r#"{"probe":"mobile"}"#)
        .send()
        .expect("mcp mobile");
    assert_eq!(response.status().as_u16(), 200);
    let body: serde_json::Value = response.json().expect("json");
    assert_eq!(body["forwarded"], true);
    assert_eq!(body["path"], "/mcp/mobile");
    assert_eq!(body["echo"], r#"{"probe":"mobile"}"#);
    let hits = mock.hits();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, "POST");
    assert_eq!(hits[0].1, "/mcp/mobile");
    stop(handle);
}

#[test]
fn get_health_is_unauthenticated_and_forwards_to_local_mcp() {
    let dir = tempfile::tempdir().expect("tmp");
    let mock = MockMcp::start();
    let handle = start_gw(mock.port, dir.path());
    let response = https_client()
        .get(gw_url(&handle, "/health"))
        .send()
        .expect("health");
    assert_eq!(response.status().as_u16(), 200);
    let body: serde_json::Value = response.json().expect("json");
    assert_eq!(body["ok"], true);
    assert_eq!(
        body["mcp"],
        format!("http://127.0.0.1:{}/mcp/<scene_slot>", mock.port)
    );
    stop(handle);
}

#[test]
fn tls_cert_is_created_once_and_survives_ip_change() {
    let dir = tempfile::tempdir().expect("tmp");
    let mock = MockMcp::start();
    with_nics(Some(vec![nic("en0", "10.1.1.1")]), || {
        let first = ensure_tls_certificate(dir.path()).expect("create cert");
        assert_eq!(first.len(), 64);
        assert!(
            first
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        let pem = std::fs::read_to_string(dir.path().join(GATEWAY_CERT_FILE)).expect("cert file");
        assert_eq!(first, sha256_hex(&pem_to_der(&pem)));
        let handle = start_gw(mock.port, dir.path());
        assert_eq!(handle.tls_fingerprint(), first);
        stop(handle);
    });
    with_nics(Some(vec![nic("en0", "10.2.2.2")]), || {
        let second = ensure_tls_certificate(dir.path()).expect("reuse cert");
        let first = sha256_hex(&pem_to_der(
            &std::fs::read_to_string(dir.path().join(GATEWAY_CERT_FILE)).expect("cert"),
        ));
        assert_eq!(second, first, "IP change must not rotate the certificate");
    });
}

#[test]
fn host_http_stays_on_loopback_and_is_not_the_lan_allowlist() {
    let local_http_src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/local_http/mod.rs"
    ));
    assert!(
        local_http_src.contains("127.0.0.1:{port}"),
        "Host HTTP must keep listening on 127.0.0.1"
    );
    assert!(
        local_http_src.contains("/api/bind-complete") && !local_http_src.contains("/mcp/mobile"),
        "Sidecar exposes bind-complete; LAN /mcp/mobile stays off Host HTTP"
    );
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let port = ephemeral_loopback_port();
    let handle = local_http::start(repo_root, port).expect("start host http");
    let probe = TcpStream::connect(format!("127.0.0.1:{port}"));
    assert!(probe.is_ok(), "Host HTTP must accept loopback");
    local_http::stop(handle);
}

#[test]
fn services_mod_registers_gateway_as_in_process_module() {
    let services = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mod.rs"
    ));
    assert!(
        services.contains("pub mod gateway"),
        "services/mod.rs must register Gateway"
    );
    let cargo = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert_eq!(
        cargo.matches("[[bin]]").count(),
        1,
        "Gateway must not be a second executable"
    );
    let local_http_src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/local_http/mod.rs"
    ));
    assert!(
        !local_http_src.contains("pub mod gateway") && !local_http_src.contains("services::gateway"),
        "Gateway must not be merged into local_http"
    );
}

#[test]
fn lan_named_routes_are_only_bind_mobile_and_health() {
    let dir = tempfile::tempdir().expect("tmp");
    let mock = MockMcp::start();
    let handle = start_gw(mock.port, dir.path());
    let client = https_client();
    for path in ["/api/status", "/foo", "/mcp/workbench", "/mcp/mobile/extra"] {
        let response = client
            .get(gw_url(&handle, path))
            .send()
            .expect("unknown path");
        assert_eq!(
            response.status().as_u16(),
            404,
            "arbitrary path {path} must not be forwarded"
        );
    }
    let get_bind = client
        .get(gw_url(&handle, "/bind/complete"))
        .send()
        .expect("get bind");
    assert_eq!(
        get_bind.status().as_u16(),
        404,
        "only POST /bind/complete is named"
    );
    assert!(
        mock.hits().is_empty(),
        "unknown LAN paths must not reach MCP"
    );
    stop(handle);
}

#[test]
fn gateway_does_not_decrypt_or_issue_tickets_or_cover_android() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/gateway/mod.rs"
    ));
    for needle in [
        "issue_for_device",
        "verify_device_token",
        "create_bind_payload",
        "ChaCha20",
        "open_bind_request",
        "android",
        "Android",
    ] {
        assert!(
            !src.contains(needle),
            "Gateway must not implement bind crypto, tickets, or Android IP update ({needle})"
        );
    }
    assert!(
        !src.contains("complete_bind") && src.contains("/api/bind-complete"),
        "POST /bind/complete must proxy ciphertext to Sidecar"
    );
}

#[test]
fn gateway_ports_stay_off_http_and_mcp_defaults() {
    assert_ne!(DEFAULT_PROD_GATEWAY_PORT, DEFAULT_PROD_HTTP_PORT);
    assert_ne!(DEFAULT_PROD_GATEWAY_PORT, DEFAULT_PROD_MCP_PORT);
    assert_ne!(DEFAULT_SANDBOX_GATEWAY_PORT, DEFAULT_SANDBOX_HTTP_PORT);
    assert_ne!(DEFAULT_SANDBOX_GATEWAY_PORT, DEFAULT_SANDBOX_MCP_PORT);
}
