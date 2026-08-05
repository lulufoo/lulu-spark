use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use super::*;

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
