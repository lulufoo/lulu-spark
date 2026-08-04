use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use super::*;
use crate::services::mcp_server_registry::{
    self, HttpMcpTransport, McpServerLookupError, SEEDED_BUSINESS_KEY,
};

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("addr")
        .port()
}

/// Minimal HTTP/1.1 stub that serves a fixed status + JSON body on any request.
fn spawn_json_stub(port: u16, status_line: &'static str, body: &'static str) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).expect("bind stub");
    listener
        .set_nonblocking(true)
        .expect("nonblocking stub listener");
    thread::spawn(move || {
        while !stop_flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = serve_one(&mut stream, status_line, body);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });
    thread::sleep(Duration::from_millis(20));
    stop
}

fn serve_one(stream: &mut TcpStream, status_line: &str, body: &str) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let mut buf = [0u8; 2048];
    let _ = stream.read(&mut buf);
    let response = format!(
        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn sample_transport(url: &str) -> HttpMcpTransport {
    HttpMcpTransport {
        name: "workbench".into(),
        url: url.into(),
        headers: std::collections::BTreeMap::from([(
            "Accept".into(),
            "application/json, text/event-stream".into(),
        )]),
    }
}

fn healthy_pair() -> (u16, u16, Arc<AtomicBool>, Arc<AtomicBool>) {
    let http_port = ephemeral_port();
    let mcp_port = ephemeral_port();
    let http_body = leak_str(format!(r#"{{"ok":true,"http_port":{http_port}}}"#));
    let mcp_body = leak_str(format!(
        r#"{{"ok":true,"mcp":"http://127.0.0.1:{mcp_port}/mcp"}}"#
    ));
    let http_stop = spawn_json_stub(http_port, "HTTP/1.1 200 OK", http_body);
    let mcp_stop = spawn_json_stub(mcp_port, "HTTP/1.1 200 OK", mcp_body);
    (http_port, mcp_port, http_stop, mcp_stop)
}

#[test]
fn static_localhost_url_alone_is_never_ready() {
    // No listeners — candidate URL must not be treated as ready.
    let port = ephemeral_port();
    let candidates = McpEndpointCandidates {
        workbench_http_base: format!("http://127.0.0.1:{port}"),
        knowledge_mcp_base: format!("http://127.0.0.1:{port}"),
        transport: sample_transport(&format!("http://127.0.0.1:{port}/mcp")),
    };
    let err = probe_mcp_endpoint_readiness(&candidates).expect_err("static URL is not ready");
    assert!(
        matches!(
            err,
            McpEndpointReadinessError::WorkbenchHttpNotReady
                | McpEndpointReadinessError::KnowledgeMcpNotReady
                | McpEndpointReadinessError::Recoverable(_)
        ),
        "expected explicit not-ready error, got {err:?}"
    );
}

#[test]
fn probe_succeeds_only_when_workbench_http_and_knowledge_mcp_are_healthy() {
    let (http_port, mcp_port, http_stop, mcp_stop) = healthy_pair();
    let transport = sample_transport(&format!("http://127.0.0.1:{mcp_port}/mcp"));
    let candidates = McpEndpointCandidates {
        workbench_http_base: format!("http://127.0.0.1:{http_port}"),
        knowledge_mcp_base: format!("http://127.0.0.1:{mcp_port}"),
        transport: transport.clone(),
    };
    let ready = probe_mcp_endpoint_readiness(&candidates).expect("both healthy → ready");
    assert_eq!(ready.transports.len(), 1);
    assert_eq!(ready.transports[0], transport);
    http_stop.store(true, Ordering::SeqCst);
    mcp_stop.store(true, Ordering::SeqCst);
}

#[test]
fn probe_fails_when_only_http_is_up() {
    let http_port = ephemeral_port();
    let mcp_port = ephemeral_port(); // nothing listening
    let http_body = leak_str(format!(r#"{{"ok":true,"http_port":{http_port}}}"#));
    let http_stop = spawn_json_stub(http_port, "HTTP/1.1 200 OK", http_body);

    let candidates = McpEndpointCandidates {
        workbench_http_base: format!("http://127.0.0.1:{http_port}"),
        knowledge_mcp_base: format!("http://127.0.0.1:{mcp_port}"),
        transport: sample_transport(&format!("http://127.0.0.1:{mcp_port}/mcp")),
    };
    let err = probe_mcp_endpoint_readiness(&candidates).expect_err("mcp down");
    assert!(
        matches!(
            err,
            McpEndpointReadinessError::KnowledgeMcpNotReady
                | McpEndpointReadinessError::Recoverable(_)
        ),
        "got {err:?}"
    );
    http_stop.store(true, Ordering::SeqCst);
}

#[test]
fn probe_fails_when_listener_is_not_workbench_service() {
    let http_port = ephemeral_port();
    let mcp_port = ephemeral_port();
    let http_stop = spawn_json_stub(http_port, "HTTP/1.1 200 OK", r#"{"hello":"world"}"#);
    let mcp_body = leak_str(format!(
        r#"{{"ok":true,"mcp":"http://127.0.0.1:{mcp_port}/mcp"}}"#
    ));
    let mcp_stop = spawn_json_stub(mcp_port, "HTTP/1.1 200 OK", mcp_body);

    let candidates = McpEndpointCandidates {
        workbench_http_base: format!("http://127.0.0.1:{http_port}"),
        knowledge_mcp_base: format!("http://127.0.0.1:{mcp_port}"),
        transport: sample_transport(&format!("http://127.0.0.1:{mcp_port}/mcp")),
    };
    let err = probe_mcp_endpoint_readiness(&candidates).expect_err("non-workbench");
    assert!(
        matches!(
            err,
            McpEndpointReadinessError::NotWorkbenchService
                | McpEndpointReadinessError::WorkbenchHttpNotReady
                | McpEndpointReadinessError::Recoverable(_)
        ),
        "got {err:?}"
    );
    http_stop.store(true, Ordering::SeqCst);
    mcp_stop.store(true, Ordering::SeqCst);
}

#[test]
fn map_to_sdk_mcp_servers_preserves_name_url_headers() {
    let transport = sample_transport("http://127.0.0.1:9876/mcp");
    let ready = ReadyMcpTransports {
        transports: vec![transport.clone()],
    };
    let mapped = map_to_sdk_mcp_servers(&ready);
    let v = serde_json::to_value(&mapped).expect("serialize sdk mcpServers");
    let entry = v
        .get(&transport.name)
        .unwrap_or_else(|| panic!("missing server key {}", transport.name));
    assert_eq!(entry.get("type").and_then(|x| x.as_str()), Some("http"));
    assert_eq!(
        entry.get("url").and_then(|x| x.as_str()),
        Some(transport.url.as_str())
    );
    let headers = entry
        .get("headers")
        .and_then(|h| h.as_object())
        .expect("headers object");
    assert_eq!(
        headers.get("Accept").and_then(|x| x.as_str()),
        Some("application/json, text/event-stream")
    );
}

#[test]
fn unknown_business_key_rejects_before_claiming_mcp_injection() {
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();

    let (http_port, mcp_port, http_stop, mcp_stop) = healthy_pair();

    let err = ready_transports_for_business_key(
        "unknown_business_key_xyz",
        http_port,
        mcp_port,
    )
    .expect_err("unknown key must reject");
    assert!(
        matches!(
            err,
            ReadyTransportError::Lookup(McpServerLookupError::NotFound)
        ),
        "must not silently degrade to empty/injected; got {err:?}"
    );

    // Known key + healthy endpoints → ready transports (not empty claim).
    let ready =
        ready_transports_for_business_key(SEEDED_BUSINESS_KEY, http_port, mcp_port).expect("seeded");
    assert!(
        !ready.transports.is_empty(),
        "known key + healthy endpoints must yield transports"
    );
    let sdk = map_to_sdk_mcp_servers(&ready);
    assert!(
        !sdk.is_empty(),
        "must not claim injection with empty mcpServers"
    );

    http_stop.store(true, Ordering::SeqCst);
    mcp_stop.store(true, Ordering::SeqCst);
    mcp_server_registry::clear_for_tests();
}

#[test]
fn readiness_module_does_not_import_cursor_sdk_or_adapter() {
    let src = include_str!("../../services/mcp_endpoint_readiness.rs");
    assert!(
        !src.contains("cursor_adapter")
            && !src.contains("@cursor/sdk")
            && !src.contains("cursor_agent_runner"),
        "Host-managed readiness must not pull Cursor SDK / adapter"
    );
}

#[test]
fn ready_transports_inject_mcp_key_path_not_bare_mcp() {
    mcp_server_registry::clear_for_tests();
    mcp_server_registry::seed_defaults();

    let (http_port, mcp_port, http_stop, mcp_stop) = healthy_pair();
    let ready =
        ready_transports_for_business_key(SEEDED_BUSINESS_KEY, http_port, mcp_port).expect("ready");
    assert_eq!(ready.transports.len(), 1);
    let url = &ready.transports[0].url;
    let expected = format!("http://127.0.0.1:{mcp_port}/mcp/{SEEDED_BUSINESS_KEY}");
    assert_eq!(
        url, &expected,
        "Binding key=todo_task must inject /mcp/todo_task, not bare /mcp"
    );
    // Regression: bare /mcp overwrite must fail.
    assert_ne!(
        url,
        &format!("http://127.0.0.1:{mcp_port}/mcp"),
        "readiness must not overwrite with bare /mcp"
    );
    let last = url.rsplit('/').next().expect("url path segment");
    assert_eq!(
        last, SEEDED_BUSINESS_KEY,
        "injected URL path last segment must equal Binding key (key≡scene_slot)"
    );

    http_stop.store(true, Ordering::SeqCst);
    mcp_stop.store(true, Ordering::SeqCst);
    mcp_server_registry::clear_for_tests();
}

#[test]
fn readiness_preserves_registry_mcp_key_path_when_rebinding_port() {
    mcp_server_registry::clear_for_tests();
    let key = "todo_task";
    let mut headers = std::collections::BTreeMap::new();
    headers.insert(
        "Accept".into(),
        "application/json, text/event-stream".into(),
    );
    mcp_server_registry::register(
        key,
        mcp_server_registry::McpServerConfig {
            capability_description: "path slot fixture".into(),
            http_transport: HttpMcpTransport {
                name: "workbench".into(),
                // Registry already carries /mcp/<key>; readiness must keep the path.
                url: format!("http://127.0.0.1:1/mcp/{key}"),
                headers,
            },
        },
    )
    .expect("register");

    let (http_port, mcp_port, http_stop, mcp_stop) = healthy_pair();
    let ready = ready_transports_for_business_key(key, http_port, mcp_port).expect("ready");
    assert_eq!(
        ready.transports[0].url,
        format!("http://127.0.0.1:{mcp_port}/mcp/{key}"),
        "must rebind port but keep /mcp/<key>; must not strip to bare /mcp"
    );

    http_stop.store(true, Ordering::SeqCst);
    mcp_stop.store(true, Ordering::SeqCst);
    mcp_server_registry::clear_for_tests();
}

#[test]
fn host_loop_still_does_not_import_cursor_sdk() {
    // Preserve Host read-only consumption boundary: loop may read McpServerConfig,
    // but must not import Cursor SDK.
    let loop_src = include_str!("../../services/agent/loop.rs");
    let llm_src = include_str!("../../services/agent/llm.rs");
    for (label, src) in [("loop.rs", loop_src), ("llm.rs", llm_src)] {
        assert!(
            !src.contains("cursor_adapter") && !src.contains("@cursor/sdk"),
            "{label} must stay Host-only (no Cursor SDK)"
        );
    }
}
