use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use super::*;
use crate::services::local_http;

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

fn http_get(url: &str) -> (u16, String) {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("http client");
    let response = client.get(url).send().expect("GET");
    let status = response.status().as_u16();
    let body = response.text().expect("body");
    (status, body)
}

/// Boundary: Cargo must pin `rmcp` on the 3.1.x line (decision verify point 3.1.0).
#[test]
fn cargo_toml_pins_rmcp_to_3_1_x() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let rmcp_line = manifest
        .lines()
        .find(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("rmcp ") || trimmed.starts_with("rmcp=")
        })
        .expect("Cargo.toml must declare an rmcp dependency");
    assert!(
        rmcp_line.contains("3.1."),
        "rmcp must be pinned to 3.1.x, got: {rmcp_line}"
    );
    assert!(
        !rmcp_line.contains("version = \"4.")
            && !rmcp_line.contains("version = \"2.")
            && !rmcp_line.contains("version = \"1."),
        "rmcp must not resolve to a non-3.1 major line, got: {rmcp_line}"
    );
}

/// Normal: Host MCP listen scaffold binds a localhost socket (ephemeral port in tests).
#[test]
fn start_mcp_listener_binds_localhost_ephemeral_port() {
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().expect("parse bind addr");
    let handle = start_mcp_listener(bind_addr, "/mcp/mvp").expect("bind MCP listen scaffold");
    let local = handle.local_addr();
    assert_eq!(local.ip().to_string(), "127.0.0.1");
    assert_ne!(local.port(), 0, "OS must assign a concrete port");

    TcpStream::connect_timeout(&local, Duration::from_millis(500))
        .expect("listener must accept TCP connections while handle is alive");

    drop(handle);

    // Port must be releasable after drop (best-effort; brief settle for OS).
    std::thread::sleep(Duration::from_millis(50));
    let rebind = std::net::TcpListener::bind(local);
    assert!(
        rebind.is_ok(),
        "dropping McpListenHandle must release the listen socket: {rebind:?}"
    );
}

/// Normal: Streamable HTTP service is nested under `/mcp/<scene_slot>` and reachable.
#[test]
fn streamable_http_nested_under_mcp_scene_slot_is_reachable() {
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().expect("parse bind addr");
    let nest_path = "/mcp/mvp";
    let handle = start_mcp_listener(bind_addr, nest_path).expect("start nested MCP scaffold");
    let local = handle.local_addr();

    // Minimal HTTP probe: a POST to the nested MCP path must get an HTTP response
    // (status line), proving the nest mount is live — not connection-refused / bare TCP.
    let mut stream = TcpStream::connect_timeout(&local, Duration::from_millis(500))
        .expect("connect to MCP listener");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t1-scaffold","version":"0.0.1"}}}"#;
    let req = format!(
        "POST {nest_path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        local.port(),
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write HTTP request");

    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    let response = String::from_utf8_lossy(&buf);
    assert!(
        response.starts_with("HTTP/1."),
        "nested MCP path must return an HTTP response, got: {response:?}"
    );
    assert!(
        !response.starts_with("HTTP/1.1 404"),
        "nested path {nest_path} must be mounted (not 404), got: {response:?}"
    );
}

/// Normal: GET /health returns JSON with ok:true and non-empty mcp (Node contract).
#[test]
fn get_health_returns_ok_true_and_nonempty_mcp() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr,
        nest_path: "/mcp/mvp".into(),
    })
    .expect("start embedded MCP runtime");
    let local = handle.local_addr();

    let (status, body) = http_get(&format!("http://127.0.0.1:{}/health", local.port()));
    assert_eq!(status, 200, "health status, body={body}");
    let json: serde_json::Value = serde_json::from_str(&body).expect("health JSON");
    assert_eq!(json.get("ok"), Some(&serde_json::Value::Bool(true)));
    let mcp = json
        .get("mcp")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(!mcp.trim().is_empty(), "mcp field must be non-empty, got {json}");

    stop_embedded_mcp_runtime(handle).expect("stop runtime");
}

/// Normal: same Host process can observe Sidecar tiny_http and MCP listen concurrently.
#[test]
fn dual_listen_sidecar_and_mcp_observable_in_same_process() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let http_port = ephemeral_port();
    let http_handle = local_http::start(repo_root, http_port).expect("start sidecar");
    thread::sleep(Duration::from_millis(50));

    let mcp_port = ephemeral_port();
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{mcp_port}").parse().expect("mcp addr"),
        nest_path: "/mcp/mvp".into(),
    })
    .expect("start MCP runtime");
    let mcp_local = mcp_handle.local_addr();

    let (sidecar_status, sidecar_body) =
        http_get(&format!("http://127.0.0.1:{http_port}/api/status"));
    assert_eq!(sidecar_status, 200, "sidecar status body={sidecar_body}");
    let sidecar_json: serde_json::Value =
        serde_json::from_str(&sidecar_body).expect("sidecar JSON");
    assert_eq!(sidecar_json.get("ok"), Some(&serde_json::Value::Bool(true)));

    let (mcp_status, mcp_body) =
        http_get(&format!("http://127.0.0.1:{}/health", mcp_local.port()));
    assert_eq!(mcp_status, 200, "mcp health body={mcp_body}");
    let mcp_json: serde_json::Value = serde_json::from_str(&mcp_body).expect("mcp JSON");
    assert_eq!(mcp_json.get("ok"), Some(&serde_json::Value::Bool(true)));
    assert!(
        mcp_json
            .get("mcp")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false),
        "mcp health mcp field empty: {mcp_json}"
    );

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    local_http::stop(http_handle);
}

/// Normal: Host teardown path can stop/join MCP runtime and release the port.
#[test]
fn stop_embedded_mcp_runtime_joins_and_releases_port() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr,
        nest_path: "/mcp/mvp".into(),
    })
    .expect("start");
    let local = handle.local_addr();

    stop_embedded_mcp_runtime(handle).expect("stop/join");
    thread::sleep(Duration::from_millis(50));

    let rebind = TcpListener::bind(local);
    assert!(
        rebind.is_ok(),
        "stop_embedded_mcp_runtime must release listen socket: {rebind:?}"
    );
}

/// Boundary: stopping MCP leaves Sidecar tiny_http thread serving; they are not the same stack.
#[test]
fn stopping_mcp_leaves_sidecar_tiny_http_serving() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let http_port = ephemeral_port();
    let http_handle = local_http::start(repo_root, http_port).expect("start sidecar");
    thread::sleep(Duration::from_millis(50));

    let mcp_port = ephemeral_port();
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{mcp_port}").parse().expect("mcp addr"),
        nest_path: "/mcp/mvp".into(),
    })
    .expect("start MCP");

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP only");

    let (status, body) = http_get(&format!("http://127.0.0.1:{http_port}/api/status"));
    assert_eq!(
        status, 200,
        "Sidecar tiny_http must keep serving after MCP stop, body={body}"
    );

    local_http::stop(http_handle);
}

/// Exception: MCP bind failure returns Err (fail-closed); must not pretend dual-listen succeeded.
#[test]
fn start_embedded_mcp_runtime_fails_closed_when_port_busy() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let _holder = TcpListener::bind(bind_addr).expect("occupy port");

    let result = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr,
        nest_path: "/mcp/mvp".into(),
    });
    assert!(
        result.is_err(),
        "busy port must fail closed (no silent dual-listen success)"
    );
}
