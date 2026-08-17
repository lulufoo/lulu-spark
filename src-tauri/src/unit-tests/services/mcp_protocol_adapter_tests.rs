use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use super::*;
use crate::services::local_http;
use crate::test_support::TestSandbox;
use rmcp::{
    ServiceExt,
    model::{
        CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation,
    },
    transport::StreamableHttpClientTransport,
};

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

fn http_post_json(url: &str, body: &str) -> (u16, String, Option<String>) {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("http client");
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(body.to_string())
        .send()
        .expect("POST");
    let status = response.status().as_u16();
    let session = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = response.text().expect("body");
    (status, body, session)
}

/// Corpus tools from Node `buildServer()` when `includeCorpus` is true.
const CORPUS_TOOLS: &[&str] = &[
    "get_corpus_catalog",
    "get_corpus_files",
    "archive_document",
    "archive_digest",
];

/// Todo tools from Node `buildServer()` when `includeTodo` is true.
const TODO_TOOLS: &[&str] = &[
    "create_todo_task",
    "update_todo_task",
    "list_todo_tasks",
    "list_todo_categories",
    "get_todo_task",
    "delete_todo_task",
    "add_todo_sub",
    "update_todo_sub",
    "delete_todo_sub",
    "complete_todo",
    "link_todo_archive",
    "add_todo_attachment",
    "list_todo_attachments",
    "get_todo_attachment",
    "update_todo_attachment",
];

/// tool name → Sidecar `/api/*` path transplanted from Node `registerTool` handlers.
fn expected_api_path(tool: &str) -> (&'static str, HttpMethod) {
    match tool {
        "get_corpus_catalog" => ("/api/corpus-catalog", HttpMethod::Get),
        "get_corpus_files" => ("/api/corpus-files", HttpMethod::Post),
        "archive_document" => ("/api/archive-document", HttpMethod::Post),
        "archive_digest" => ("/api/archive-digest", HttpMethod::Post),
        "create_todo_task" => ("/api/todo-task-create", HttpMethod::Post),
        "update_todo_task" => ("/api/todo-task-update", HttpMethod::Post),
        "list_todo_tasks" => ("/api/todo-tasks", HttpMethod::Get),
        "list_todo_categories" => ("/api/todo-task-list-categories", HttpMethod::Get),
        "get_todo_task" => ("/api/todo-task", HttpMethod::Get),
        "delete_todo_task" => ("/api/todo-task-delete", HttpMethod::Post),
        "add_todo_sub" => ("/api/todo-task-add-sub", HttpMethod::Post),
        "update_todo_sub" => ("/api/todo-task-update-sub", HttpMethod::Post),
        "delete_todo_sub" => ("/api/todo-task-delete-sub", HttpMethod::Post),
        "complete_todo" => ("/api/todo-task-complete", HttpMethod::Post),
        "link_todo_archive" => ("/api/todo-task-link-archive", HttpMethod::Post),
        "add_todo_attachment" => ("/api/todo-task-add-attachment", HttpMethod::Post),
        "list_todo_attachments" => ("/api/todo-task-list-attachments", HttpMethod::Post),
        "get_todo_attachment" => ("/api/todo-task-get-attachment", HttpMethod::Post),
        "update_todo_attachment" => ("/api/todo-task-update-attachment", HttpMethod::Post),
        "get_notes_selection" => ("/api/notes-selection", HttpMethod::Get),
        other => panic!("unexpected tool in fixture: {other}"),
    }
}

/// Read-only notes selection tool (t1 Sidecar GET /api/notes-selection).
const NOTES_SELECTION_TOOL: &str = "get_notes_selection";

fn names_of(tools: &[ToolDescriptor]) -> BTreeSet<String> {
    tools.iter().map(|t| t.name.clone()).collect()
}

fn route_map(table: &SlotToolTable) -> std::collections::BTreeMap<String, (String, HttpMethod)> {
    table
        .tools
        .iter()
        .map(|r| (r.name.clone(), (r.api_path.clone(), r.method)))
        .collect()
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
    let handle = start_mcp_listener(bind_addr).expect("bind MCP listen scaffold");
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
    let nest_path = "/mcp/todo_task";
    let handle = start_mcp_listener(bind_addr).expect("start nested MCP scaffold");
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
    let handle =
        start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start embedded MCP runtime");
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
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
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

/// Normal: free port → MCP runtime binds and starts (Host :9876 contract shape).
#[test]
fn start_embedded_mcp_runtime_binds_when_port_free() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");

    let handle =
        start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("bind free port");
    assert_eq!(handle.local_addr().ip().to_string(), "127.0.0.1");
    assert_eq!(handle.local_addr().port(), port);

    let (status, body) = http_get(&format!("http://127.0.0.1:{port}/health"));
    assert_eq!(status, 200, "health after bind, body={body}");

    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Exception: MCP bind failure returns Err (fail-closed) with a clear BindFailed error.
#[test]
fn start_embedded_mcp_runtime_fails_closed_when_port_busy() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let _holder = TcpListener::bind(bind_addr).expect("occupy port");

    let err = match start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }) {
        Ok(_) => panic!("busy port must fail closed (no silent dual-listen success)"),
        Err(e) => e,
    };
    assert!(
        matches!(err, McpStartError::BindFailed(_)),
        "bind conflict must be BindFailed, got {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("MCP bind failed"),
        "error must be clear/prefixed, got {msg:?}"
    );
    assert!(
        msg.len() > "MCP bind failed: ".len(),
        "error must include underlying bind reason, got {msg:?}"
    );
}

/// Normal: `todo_task` routing table = todo tools only (SCENE_SLOT_API includeCorpus=false).
#[test]
fn build_slot_tool_table_todo_task_matches_node_allowlist() {
    let table = build_slot_tool_table("todo_task").expect("todo_task registered");
    assert_eq!(table.scene_slot, "todo_task");
    assert!(!table.include_corpus);
    assert!(table.include_todo);

    let names: BTreeSet<_> = table.tools.iter().map(|t| t.name.as_str()).collect();
    let expected: BTreeSet<_> = TODO_TOOLS.iter().copied().collect();
    assert_eq!(names, expected, "todo_task tools must match Node includeTodo set");

    let routes = route_map(&table);
    for tool in TODO_TOOLS {
        let (path, method) = expected_api_path(tool);
        let got = routes.get(*tool).expect("route present");
        assert_eq!(got.0, path, "api path for {tool}");
        assert_eq!(got.1, method, "http method for {tool}");
        assert!(
            got.0.starts_with("/api/"),
            "outbound target must be Sidecar /api/* path, got {}",
            got.0
        );
    }
}

/// Normal: `cursor_ide` routing table = corpus + todo (SCENE_SLOT_API both true).
#[test]
fn build_slot_tool_table_cursor_ide_matches_node_allowlist() {
    let table = build_slot_tool_table("cursor_ide").expect("cursor_ide registered");
    assert_eq!(table.scene_slot, "cursor_ide");
    assert!(table.include_corpus);
    assert!(table.include_todo);

    let names: BTreeSet<_> = table.tools.iter().map(|t| t.name.as_str()).collect();
    let mut expected: BTreeSet<_> = CORPUS_TOOLS.iter().copied().collect();
    expected.extend(TODO_TOOLS.iter().copied());
    assert_eq!(names, expected, "cursor_ide tools must match Node corpus+todo set");

    let routes = route_map(&table);
    for tool in expected.iter() {
        let (path, method) = expected_api_path(tool);
        let got = routes.get(*tool).expect("route present");
        assert_eq!(got.0, path, "api path for {tool}");
        assert_eq!(got.1, method, "http method for {tool}");
    }
}

/// Boundary: includeCorpus=false shrinks list vs cursor_ide (Node SCENE_SLOT_API switch).
#[test]
fn tools_list_contracts_when_corpus_switch_off() {
    let todo_names = names_of(&tools_list_for_slot("todo_task"));
    let ide_names = names_of(&tools_list_for_slot("cursor_ide"));

    assert!(!todo_names.is_empty());
    assert!(!ide_names.is_empty());
    assert!(
        ide_names.is_superset(&todo_names),
        "cursor_ide must be a superset of todo_task when both includeTodo"
    );
    for corpus in CORPUS_TOOLS {
        assert!(
            !todo_names.contains(*corpus),
            "todo_task must omit corpus tool {corpus} (includeCorpus=false)"
        );
        assert!(
            ide_names.contains(*corpus),
            "cursor_ide must include corpus tool {corpus}"
        );
    }
    assert_ne!(
        todo_names, ide_names,
        "todo_task vs cursor_ide tool surfaces must be distinguishable"
    );
}

/// Exception: unregistered slot yields None (never enters proxy/allowlist handlers).
#[test]
fn build_slot_tool_table_unknown_slot_returns_none() {
    assert!(build_slot_tool_table("__unknown__").is_none());
    assert!(build_slot_tool_table("").is_none());
    assert!(tools_list_for_slot("__unknown__").is_empty());
}

/// Exception: unknown scene_slot hard-rejects at HTTP/routing layer with NO MCP session.
#[test]
fn unknown_scene_slot_http_hard_reject_without_mcp_session() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t3-unknown","version":"0.0.1"}}}"#;
    let (status, body, session) = http_post_json(
        &format!("http://127.0.0.1:{}/mcp/__unknown__", local.port()),
        init,
    );

    assert_eq!(
        status, 404,
        "unknown slot must HTTP hard-reject, body={body}"
    );
    let json: serde_json::Value = serde_json::from_str(&body).expect("reject JSON");
    assert_eq!(
        json.get("error").and_then(|v| v.as_str()),
        Some("unknown_scene_slot"),
        "reject body={body}"
    );
    assert_eq!(
        json.get("scene_slot").and_then(|v| v.as_str()),
        Some("__unknown__")
    );
    assert!(
        session.is_none(),
        "must not establish MCP session for unknown slot, got session={session:?}"
    );
    assert!(
        json.get("result").is_none(),
        "must not return MCP initialize result; got {body}"
    );

    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Exception: bare `/mcp` (no scene_slot) hard-rejects; not a registered isolation surface.
#[test]
fn bare_mcp_path_http_hard_reject_without_mcp_session() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t3-bare","version":"0.0.1"}}}"#;
    let (status, body, session) =
        http_post_json(&format!("http://127.0.0.1:{}/mcp", local.port()), init);

    assert_eq!(status, 404, "bare /mcp must hard-reject, body={body}");
    let json: serde_json::Value = serde_json::from_str(&body).expect("reject JSON");
    assert_eq!(
        json.get("error").and_then(|v| v.as_str()),
        Some("scene_slot_required"),
        "reject body={body}"
    );
    assert!(session.is_none(), "bare /mcp must not create MCP session");

    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Normal: registered slots remain reachable (not only tools/list contract).
#[test]
fn registered_scene_slots_accept_initialize_http() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t3-registered","version":"0.0.1"}}}"#;
    for slot in ["todo_task", "cursor_ide"] {
        let (status, body, _) = http_post_json(
            &format!("http://127.0.0.1:{}/mcp/{slot}", local.port()),
            init,
        );
        assert_ne!(
            status, 404,
            "registered slot {slot} must not hard-reject, body={body}"
        );
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            assert_ne!(
                json.get("error").and_then(|v| v.as_str()),
                Some("unknown_scene_slot"),
                "registered slot {slot} must not return unknown_scene_slot"
            );
        }
    }

    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Observable text from Mapped MCP tool result/error (Node content[0].text).
fn mcp_text_ok(r: &McpToolResult) -> &str {
    &r.content_text
}

fn mcp_text_err(e: &McpToolError) -> &str {
    &e.content_text
}

/// Normal: Sidecar 2xx maps to MCP tool result text (Node: content text = body).
#[test]
fn map_sidecar_success_response_matches_node_adapter_shape() {
    let mapped = map_sidecar_response_to_mcp(200, br#"{"ok":true,"items":[]}"#)
        .expect("2xx must be success");
    assert_eq!(mcp_text_ok(&mapped), r#"{"ok":true,"items":[]}"#);
    let call = mapped_to_call_tool_result(Ok(mapped));
    assert_eq!(call.is_error, Some(false));
    assert_eq!(
        call.content[0].as_text().map(|t| t.text.as_str()),
        Some(r#"{"ok":true,"items":[]}"#)
    );
}

/// Exception: Sidecar non-2xx maps to Node-equivalent tool error `HTTP {status}: {text}`.
#[test]
fn map_sidecar_error_response_matches_node_adapter_shape() {
    let err = map_sidecar_response_to_mcp(404, b"{\"error\":\"missing\"}")
        .expect_err("non-2xx must be tool error");
    assert_eq!(
        mcp_text_err(&err),
        "HTTP 404: {\"error\":\"missing\"}"
    );
    let call = mapped_to_call_tool_result(Err(err));
    assert_eq!(call.is_error, Some(true));
    assert_eq!(
        call.content[0].as_text().map(|t| t.text.as_str()),
        Some("HTTP 404: {\"error\":\"missing\"}")
    );
}

/// Exception: Sidecar unreachable maps like Node toolError(503, "Workbench HTTP unreachable: …").
#[test]
fn proxy_tool_call_unreachable_sidecar_maps_like_node() {
    // Nothing listens on this port.
    let base = "http://127.0.0.1:1";
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let mapped = rt
        .block_on(proxy_tool_call(
            base,
            "todo_task",
            "list_todo_categories",
            serde_json::json!({}),
        ))
        .expect("unreachable is a mapped tool error, not protocol failure");
    let err = mapped.expect_err("must be tool-level error");
    let text = mcp_text_err(&err);
    assert!(
        text.starts_with("HTTP 503: Workbench HTTP unreachable:"),
        "Node-equivalent unreachable prefix, got {text}"
    );
}

/// Start a tiny recording Sidecar on loopback; returns (base_url, join, seen Arc).
fn start_recording_sidecar(
    expected_status: u16,
    response_body: &'static str,
) -> (
    String,
    std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
    thread::JoinHandle<()>,
) {
    use std::sync::{Arc, Mutex};
    use tiny_http::{Header, Response, Server, StatusCode};

    let port = ephemeral_port();
    let server = Server::http(format!("127.0.0.1:{port}")).expect("bind recording sidecar");
    let base = format!("http://127.0.0.1:{port}");
    let seen: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let seen_thread = Arc::clone(&seen);
    let join = thread::spawn(move || {
        // One request is enough for representative smoke; timeout avoids join hang.
        let Ok(Some(mut request)) = server.recv_timeout(Duration::from_secs(5)) else {
            return;
        };
        let method = request.method().as_str().to_string();
        let url = request.url().to_string();
        seen_thread.lock().expect("lock").push((method, url));
        let status = StatusCode::from(expected_status);
        let mut response = Response::from_string(response_body).with_status_code(status);
        response.add_header(
            Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        );
        let _ = request.respond(response);
    });
    // Brief settle for accept loop.
    thread::sleep(Duration::from_millis(20));
    (base, seen, join)
}

/// Normal: each registered slot gets ≥1 representative tools/call via HTTP loopback to /api/*.
#[test]
fn representative_tools_call_per_registered_slot_hits_sidecar_api() {
    let cases = [
        (
            "todo_task",
            "list_todo_categories",
            serde_json::json!({}),
            "GET",
            "/api/todo-task-list-categories",
        ),
        (
            "cursor_ide",
            "get_corpus_catalog",
            serde_json::json!({"mode": "latest_per_topic"}),
            "GET",
            "/api/corpus-catalog?mode=latest_per_topic",
        ),
    ];

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    for (slot, tool, args, want_method, want_path) in cases {
        let body = r#"{"ok":true,"proxy":"sidecar"}"#;
        let (base, seen, join) = start_recording_sidecar(200, body);
        let mapped = rt
            .block_on(proxy_tool_call(&base, slot, tool, args))
            .expect("proxy must return mapped result")
            .expect("Sidecar 200 → MCP success");
        assert_eq!(mcp_text_ok(&mapped), body);

        let hits = seen.lock().expect("lock").clone();
        assert!(
            hits.iter().any(|(m, u)| m == want_method && u == want_path),
            "slot={slot} tool={tool} expected {want_method} {want_path}, seen={hits:?}"
        );
        // Ensure path is under /api/* (Sidecar surface).
        assert!(
            hits.iter().any(|(_, u)| u.starts_with("/api/")),
            "must hit Sidecar /api/*, seen={hits:?}"
        );
        let _ = join.join();
    }
}

/// Exception: Sidecar error body via proxy_tool_call stays Node-equivalent.
#[test]
fn proxy_tool_call_maps_sidecar_http_error_like_node() {
    let (base, _seen, join) = start_recording_sidecar(500, "boom");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let err = rt
        .block_on(proxy_tool_call(
            &base,
            "todo_task",
            "list_todo_categories",
            serde_json::json!({}),
        ))
        .expect("mapped")
        .expect_err("500 → tool error");
    assert_eq!(mcp_text_err(&err), "HTTP 500: boom");
    let _ = join.join();
}

/// Exception: Adapter source must not reach FS or domain modules directly (loopback only).
#[test]
fn adapter_source_forbids_fs_and_domain_direct_access() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mcp_protocol_adapter.rs"
    ));
    for needle in [
        "std::fs",
        "tokio::fs",
        "crate::services::todo_task",
        "crate::services::workbench_read",
        "crate::services::archive_write",
        "crate::services::kb",
        "crate::repositories",
    ] {
        assert!(
            !src.contains(needle),
            "Adapter must not {needle}; domain/Corpus only via Sidecar HTTP"
        );
    }
    assert!(
        src.contains("127.0.0.1") && src.contains("8765"),
        "Adapter must default Sidecar loopback to 127.0.0.1:8765"
    );
    assert!(
        src.contains("/api/") || src.contains("api_path"),
        "Adapter outbound targets must be Sidecar /api/* routes"
    );
}

/// Production close-gate ports (V5 / Topic2): MCP `:9876` + Sidecar `:8765`.
const CLOSE_GATE_MCP_PORT: u16 = 9876;
const CLOSE_GATE_SIDECAR_PORT: u16 = 8765;

fn start_close_gate_dual_listen() -> (local_http::LocalHttpHandle, McpRuntimeHandle) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let http_handle =
        local_http::start(repo_root, CLOSE_GATE_SIDECAR_PORT).expect("start Sidecar :8765");
    thread::sleep(Duration::from_millis(50));
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{CLOSE_GATE_MCP_PORT}")
            .parse()
            .expect("mcp addr"),
    })
    .expect("start MCP :9876");
    (http_handle, mcp_handle)
}

/// Normal / V5: same-process dual listen (:9876 + :8765) + registered-slot initialize + tools/list.
#[test]
fn close_gate_smoke_initialize_list_passes_v5_dual_listen_and_session() {
    let (http_handle, mcp_handle) = start_close_gate_dual_listen();

    let report = close_gate_smoke_initialize_list("todo_task")
        .expect("P1 close gate must pass before Node spawn hard-cut");

    assert!(
        report.dual_listen_observed,
        "same-process :9876 + :8765 must be observable, report={report:?}"
    );
    assert_eq!(report.mcp_port, CLOSE_GATE_MCP_PORT);
    assert_eq!(report.sidecar_port, CLOSE_GATE_SIDECAR_PORT);
    assert!(
        report.initialize_ok,
        "Streamable HTTP initialize must succeed on registered slot"
    );
    assert!(
        report.tools_list_ok,
        "tools/list must succeed on registered slot after initialize"
    );
    assert!(
        !report.tool_names.is_empty(),
        "tools/list must return at least one tool name"
    );
    assert!(
        report
            .tool_names
            .iter()
            .any(|n| n == "list_todo_categories"),
        "todo_task tools/list must include list_todo_categories, got {:?}",
        report.tool_names
    );

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    local_http::stop(http_handle);
}

/// Boundary: readiness `/health` success ≠ session-level initialize/tools/list (T2/T8 vs T6).
#[test]
fn health_success_is_not_session_level_close_gate_proof() {
    let (http_handle, mcp_handle) = start_close_gate_dual_listen();

    let (status, health_body) =
        http_get(&format!("http://127.0.0.1:{CLOSE_GATE_MCP_PORT}/health"));
    assert_eq!(status, 200, "health body={health_body}");
    let health_json: serde_json::Value =
        serde_json::from_str(&health_body).expect("health JSON");
    assert_eq!(
        health_json.get("ok"),
        Some(&serde_json::Value::Bool(true))
    );
    // /health contract has no tools surface — session proof is a separate close-gate step.
    assert!(
        health_json.get("tools").is_none(),
        "health must not carry tools/list payload: {health_json}"
    );
    assert!(
        !health_body.contains("list_todo_categories"),
        "health body must not embed tools/list names"
    );

    let report = close_gate_smoke_initialize_list("todo_task").expect("session close gate");
    assert!(
        report.tools_list_ok && !report.tool_names.is_empty(),
        "session initialize+tools/list is the close-gate proof, not /health alone"
    );

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    local_http::stop(http_handle);
}

/// Exception / T7: after V5 close gate, P2 hard-cut must remove Node spawn lifecycle paths.
#[test]
fn p2_hard_cut_must_remove_node_spawn_lifecycle_paths() {
    let lib_src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    for needle in [
        "fn try_spawn_knowledge_mcp(",
        "fn try_spawn_knowledge_mcp_with_port(",
        "fn try_spawn_knowledge_mcp_with_port_and_node(",
        "struct KnowledgeMcpProcess",
        "packages/knowledge-mcp/index.mjs",
    ] {
        assert!(
            !lib_src.contains(needle),
            "T7 P2 hard-cut: Node spawn path `{needle}` must be removed from Host lib.rs (V1)"
        );
    }
}

/// Exception: unregistered slot cannot satisfy the close gate.
#[test]
fn close_gate_smoke_rejects_unregistered_slot() {
    let (http_handle, mcp_handle) = start_close_gate_dual_listen();

    let err = close_gate_smoke_initialize_list("__unknown__")
        .expect_err("unregistered slot must fail close gate");
    match err {
        CloseGateError::UnregisteredSlot(slot) => assert_eq!(slot, "__unknown__"),
        other => panic!("expected UnregisteredSlot, got {other:?}"),
    }

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    local_http::stop(http_handle);
}


/// T10 / V4: Node `packages/knowledge-mcp/index.mjs` must not remain a runtime SSOT.
#[test]
fn p3_t10_knowledge_mcp_package_not_runtime_ssot() {
    let index = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../packages/knowledge-mcp/index.mjs");
    assert!(
        !index.exists(),
        "T10/V4: packages/knowledge-mcp/index.mjs must be archived/removed (not runtime SSOT); found {}",
        index.display()
    );
    let pkg_json =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../packages/knowledge-mcp/package.json");
    assert!(
        !pkg_json.exists(),
        "T10/V4: packages/knowledge-mcp must not remain as an installable runtime package"
    );
}

/// T10 / V2+V3: dual-slot tools/list + representative tools/call + unknown-slot hard-fail
/// against Host MCP URL `http://127.0.0.1:9876` (Sidecar fixture stays process-independent).
#[test]
fn p3_t10_host_dual_slot_list_call_and_unknown_hard_fail_smoke() {
    // Sidecar HTTP fixture: independent local_http on :8765 with migration gate planted.
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let todo_root = wb.join("todo_tasks");
    fs::create_dir_all(&todo_root).expect("mkdir todo_tasks");
    fs::write(todo_root.join(".migration_gate_passed"), b"ok\n").expect("plant migration gate");
    fs::write(wb.join("index.json"), br#"{"entries":{}}"#).expect("plant empty corpus index");

    let http_handle =
        local_http::start(sandbox.config_dir().to_path_buf(), CLOSE_GATE_SIDECAR_PORT)
            .expect("start Sidecar :8765 independent of MCP");
    thread::sleep(Duration::from_millis(50));
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{CLOSE_GATE_MCP_PORT}")
            .parse()
            .expect("mcp addr"),
    })
    .expect("start Host MCP :9876");

    let (sidecar_status, _) = http_get(&format!(
        "http://127.0.0.1:{CLOSE_GATE_SIDECAR_PORT}/api/status"
    ));
    assert_eq!(
        sidecar_status, 200,
        "T10: Sidecar HTTP fixture must be independently observable on :8765"
    );

    let (mcp_health_status, mcp_health_body) =
        http_get(&format!("http://127.0.0.1:{CLOSE_GATE_MCP_PORT}/health"));
    assert_eq!(
        mcp_health_status, 200,
        "T10: Host MCP /health must succeed on :9876, body={mcp_health_body}"
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");

    let todo_names = rt
        .block_on(super::run_initialize_and_list_tools(
            CLOSE_GATE_MCP_PORT,
            "todo_task",
        ))
        .expect("todo_task tools/list on Host :9876");
    for tool in ["list_todo_tasks", "create_todo_task"] {
        assert!(
            todo_names.iter().any(|n| n == tool),
            "T10/V2: todo_task missing {tool}; got {todo_names:?}"
        );
    }
    for tool in CORPUS_TOOLS {
        assert!(
            !todo_names.iter().any(|n| n == *tool),
            "T10/V2: todo_task must not expose corpus tool {tool}"
        );
    }

    let ide_names = rt
        .block_on(super::run_initialize_and_list_tools(
            CLOSE_GATE_MCP_PORT,
            "cursor_ide",
        ))
        .expect("cursor_ide tools/list on Host :9876");
    for tool in CORPUS_TOOLS {
        assert!(
            ide_names.iter().any(|n| n == *tool),
            "T10/V2: cursor_ide missing {tool}; got {ide_names:?}"
        );
    }
    assert_ne!(
        todo_names, ide_names,
        "T10/V2: todo_task and cursor_ide tools/list must be distinguishable"
    );

    async fn list_and_call(
        port: u16,
        slot: &str,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<(Vec<String>, bool, String), String> {
        let url = format!("http://127.0.0.1:{port}/mcp/{slot}");
        let transport = StreamableHttpClientTransport::from_uri(url);
        let client_info = ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("t10-host-smoke", "0.1.0"),
        );
        let client = client_info
            .serve(transport)
            .await
            .map_err(|e| format!("initialize {slot}: {e:#}"))?;
        let tools = client
            .list_tools(Default::default())
            .await
            .map_err(|e| format!("list_tools {slot}: {e:#}"))?;
        let names: Vec<String> = tools.tools.iter().map(|t| t.name.to_string()).collect();
        let mut params = CallToolRequestParams::new(tool.to_string());
        if let Some(obj) = args.as_object() {
            params.arguments = Some(obj.clone());
        }
        let result = client
            .call_tool(params)
            .await
            .map_err(|e| format!("call_tool {slot}/{tool}: {e:#}"))?;
        let is_error = result.is_error.unwrap_or(false);
        let text = result
            .content
            .first()
            .and_then(|c| c.as_text().map(|t| t.text.clone()))
            .unwrap_or_default();
        let _ = client.cancel().await;
        Ok((names, is_error, text))
    }

    let (_, todo_err, todo_text) = rt
        .block_on(list_and_call(
            CLOSE_GATE_MCP_PORT,
            "todo_task",
            "list_todo_tasks",
            serde_json::json!({}),
        ))
        .expect("todo_task list_todo_tasks");
    assert!(
        !todo_err,
        "T10/V2: todo_task representative tools/call must succeed via Host→Sidecar; got {todo_text}"
    );

    let (_, ide_err, ide_text) = rt
        .block_on(list_and_call(
            CLOSE_GATE_MCP_PORT,
            "cursor_ide",
            "get_corpus_catalog",
            serde_json::json!({"mode": "latest_per_topic"}),
        ))
        .expect("cursor_ide get_corpus_catalog");
    assert!(
        !ide_err,
        "T10/V2: cursor_ide representative tools/call must succeed via Host→Sidecar; got {ide_text}"
    );

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t10-unknown","version":"0.0.1"}}}"#;
    let (status, body, session) = http_post_json(
        &format!("http://127.0.0.1:{CLOSE_GATE_MCP_PORT}/mcp/__unknown__"),
        init,
    );
    assert_eq!(
        status, 404,
        "T10/V3: unknown slot must HTTP hard-reject on Host :9876, body={body}"
    );
    assert!(
        session.is_none(),
        "T10/V3: unknown slot must not establish MCP session"
    );

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    local_http::stop(http_handle);
    drop(sandbox);
}

fn notes_expected_tool_names() -> BTreeSet<&'static str> {
    let mut names: BTreeSet<&'static str> = CORPUS_TOOLS.iter().copied().collect();
    names.insert(NOTES_SELECTION_TOOL);
    names
}

fn adapter_source() -> &'static str {
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mcp_protocol_adapter.rs"
    ))
}

fn source_fn_after<'a>(src: &'a str, fn_name: &str) -> &'a str {
    src.split(&format!("fn {fn_name}"))
        .nth(1)
        .unwrap_or_else(|| panic!("{fn_name} must exist"))
}

/// Normal: notes tools/list = corpus four-pack + read-only get_notes_selection.
#[test]
fn tools_list_for_notes_is_corpus_four_pack_plus_get_notes_selection() {
    let names = names_of(&tools_list_for_slot("notes"));
    let expected: BTreeSet<_> = notes_expected_tool_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(
        names, expected,
        "notes tools/list must be corpus four-pack + get_notes_selection"
    );
}

/// Normal: notes corpus routes are the same Sidecar /api paths as cursor_ide.
#[test]
fn notes_corpus_routes_match_cursor_ide_sidecar_api() {
    let table = build_slot_tool_table("notes").expect("notes must be registered");
    let routes = route_map(&table);
    for tool in CORPUS_TOOLS {
        let (path, method) = expected_api_path(tool);
        let got = routes.get(*tool).expect("corpus route present on notes");
        assert_eq!(got.0, path, "notes must proxy the same /api path as cursor_ide for {tool}");
        assert_eq!(got.1, method, "notes must use the same HTTP method as cursor_ide for {tool}");
    }
}

/// Normal: get_notes_selection proxies t1 GET /api/notes-selection and returns the snapshot shape.
#[test]
fn get_notes_selection_proxies_sidecar_read_snapshot() {
    let body = r#"{"date":"2026-06-19","documents":[{"id":"11111111111111111111111111111111","selected":true}]}"#;
    let (base, seen, join) = start_recording_sidecar(200, body);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let mapped = rt
        .block_on(proxy_tool_call(
            &base,
            "notes",
            NOTES_SELECTION_TOOL,
            serde_json::json!({}),
        ))
        .expect("proxy must return mapped result")
        .expect("Sidecar 200 → MCP success");
    assert_eq!(mcp_text_ok(&mapped), body);

    let snapshot: serde_json::Value = serde_json::from_str(mcp_text_ok(&mapped)).expect("snapshot JSON");
    assert_eq!(snapshot["date"], "2026-06-19");
    let docs = snapshot["documents"].as_array().expect("documents");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0]["id"], "11111111111111111111111111111111");
    assert_eq!(docs[0]["selected"], true);

    let hits = seen.lock().expect("lock").clone();
    assert!(
        hits.iter()
            .any(|(m, u)| m == "GET" && u == "/api/notes-selection"),
        "get_notes_selection must GET /api/notes-selection, seen={hits:?}"
    );
    let _ = join.join();
}

/// Boundary: scene_slot_api("notes") seeds corpus only (no todo).
#[test]
fn scene_slot_api_notes_seeds_corpus_not_todo() {
    assert!(
        super::REGISTERED_SCENE_SLOTS.contains(&"notes"),
        "REGISTERED_SCENE_SLOTS must include notes"
    );
    let api = super::scene_slot_api("notes").expect("notes must be a registered slot");
    assert!(api.include_corpus, "notes must seed corpus four-pack");
    assert!(!api.include_todo, "notes must not seed todo tools");

    let table = build_slot_tool_table("notes").expect("notes table");
    assert!(table.include_corpus);
    assert!(!table.include_todo);
}

/// Boundary: get_notes_selection is a ToolRoute hung in build_slot_tool_table, not named by scene_slot_api.
#[test]
fn get_notes_selection_is_hung_in_build_slot_tool_table_not_scene_slot_api() {
    let table = build_slot_tool_table("notes").expect("notes registered");
    let route = table
        .tools
        .iter()
        .find(|r| r.name == NOTES_SELECTION_TOOL)
        .expect("build_slot_tool_table must attach get_notes_selection");
    assert_eq!(route.api_path, "/api/notes-selection");
    assert_eq!(route.method, HttpMethod::Get);
    assert!(
        !CORPUS_TOOLS.contains(&NOTES_SELECTION_TOOL),
        "get_notes_selection is not a corpus seed tool"
    );

    let src = adapter_source();
    let scene_fn = source_fn_after(src, "scene_slot_api")
        .split("fn corpus_tool_routes")
        .next()
        .expect("scene_slot_api body");
    assert!(
        !scene_fn.contains("get_notes_selection"),
        "scene_slot_api must not name a third tool"
    );
    let build_fn = source_fn_after(src, "build_slot_tool_table")
        .split("/// Allowlisted tool names")
        .next()
        .expect("build_slot_tool_table body");
    assert!(
        build_fn.contains("get_notes_selection"),
        "build_slot_tool_table must hang get_notes_selection as a ToolRoute"
    );
}

/// Boundary: notes slot contains no todo tools.
#[test]
fn notes_slot_contains_no_todo_tools() {
    let names = names_of(&tools_list_for_slot("notes"));
    for tool in TODO_TOOLS {
        assert!(
            !names.contains(*tool),
            "notes must not expose todo tool {tool}"
        );
    }
    let table = build_slot_tool_table("notes").expect("notes registered");
    assert!(!table.include_todo);
}

/// Exception: unregistered slots still return None after notes is registered.
#[test]
fn build_slot_tool_table_unregistered_still_none_when_notes_exists() {
    assert!(
        build_slot_tool_table("notes").is_some(),
        "notes must be registered so the hard-reject path is distinguishable"
    );
    assert!(build_slot_tool_table("__unknown__").is_none());
    assert!(build_slot_tool_table("").is_none());
    assert!(tools_list_for_slot("__unknown__").is_empty());
}

/// Exception: notes exposes no MCP write-selection tool.
#[test]
fn notes_slot_has_no_mcp_write_selection_tool() {
    let table = build_slot_tool_table("notes").expect("notes registered");
    for tool in &table.tools {
        let name = tool.name.to_lowercase();
        let writes_selection = name.contains("notes_selection")
            && (name.contains("put")
                || name.contains("set")
                || name.contains("write")
                || name.contains("update"));
        assert!(
            !writes_selection,
            "must not provide an MCP write-selection tool, found {}",
            tool.name
        );
        assert!(
            !(tool.api_path.contains("notes-selection") && tool.method != HttpMethod::Get),
            "notes selection MCP route must be GET-only, found {} {:?}",
            tool.name,
            tool.method
        );
    }
    assert!(
        !table.tools.iter().any(|t| t.name == "put_notes_selection"
            || t.name == "set_notes_selection"
            || t.name == "write_notes_selection"),
        "must not register a write-selection MCP tool name"
    );
}

/// Normal: /mcp/notes is mounted and tools/list matches the notes surface.
#[test]
fn notes_scene_slot_initialize_lists_corpus_plus_selection() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t3-notes","version":"0.0.1"}}}"#;
    let (status, body, _) = http_post_json(
        &format!("http://127.0.0.1:{}/mcp/notes", local.port()),
        init,
    );
    assert_ne!(
        status, 404,
        "registered notes slot must not hard-reject, body={body}"
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");
    let names = rt
        .block_on(super::run_initialize_and_list_tools(local.port(), "notes"))
        .expect("notes tools/list");
    let got: BTreeSet<_> = names.iter().map(String::as_str).collect();
    let expected = notes_expected_tool_names();
    assert_eq!(got, expected, "HTTP tools/list for /mcp/notes");
    for tool in TODO_TOOLS {
        assert!(
            !got.contains(tool),
            "HTTP tools/list for notes must omit todo tool {tool}"
        );
    }

    stop_embedded_mcp_runtime(handle).expect("stop");
}

fn notes_proprietary_tool_names() -> BTreeSet<String> {
    let notes = names_of(&tools_list_for_slot("notes"));
    let shared: BTreeSet<String> = CORPUS_TOOLS
        .iter()
        .chain(TODO_TOOLS.iter())
        .map(|s| (*s).to_string())
        .collect();
    notes.difference(&shared).cloned().collect()
}

/// P4 / T7 Normal: todo_task remains all todo tools, no corpus, original `/api` routes.
#[test]
fn p4_todo_task_surface_unchanged_all_todo_no_corpus_original_api() {
    let table = build_slot_tool_table("todo_task").expect("todo_task registered");
    assert_eq!(table.scene_slot, "todo_task");
    assert!(!table.include_corpus);
    assert!(table.include_todo);

    let names = names_of(&tools_list_for_slot("todo_task"));
    let expected: BTreeSet<_> = TODO_TOOLS.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        names, expected,
        "todo_task must stay the full todo tool set with no corpus"
    );

    let routes = route_map(&table);
    for tool in TODO_TOOLS {
        let (path, method) = expected_api_path(tool);
        let got = routes.get(*tool).expect("todo route present");
        assert_eq!(got.0, path, "todo_task {tool} must keep original /api path");
        assert_eq!(got.1, method, "todo_task {tool} must keep original method");
        assert!(
            got.0.starts_with("/api/"),
            "todo_task {tool} must still route through original /api, got {}",
            got.0
        );
    }
}

/// P4 / T7 Normal: cursor_ide remains corpus four-pack + all todo, original `/api` routes.
#[test]
fn p4_cursor_ide_surface_unchanged_corpus_plus_todo_original_api() {
    let table = build_slot_tool_table("cursor_ide").expect("cursor_ide registered");
    assert_eq!(table.scene_slot, "cursor_ide");
    assert!(table.include_corpus);
    assert!(table.include_todo);

    let names = names_of(&tools_list_for_slot("cursor_ide"));
    let mut expected: BTreeSet<_> = CORPUS_TOOLS.iter().map(|s| (*s).to_string()).collect();
    expected.extend(TODO_TOOLS.iter().map(|s| (*s).to_string()));
    assert_eq!(
        names, expected,
        "cursor_ide must stay corpus four-pack + all todo"
    );

    let routes = route_map(&table);
    for tool in expected.iter() {
        let (path, method) = expected_api_path(tool);
        let got = routes.get(tool).expect("cursor_ide route present");
        assert_eq!(got.0, path, "cursor_ide {tool} must keep original /api path");
        assert_eq!(got.1, method, "cursor_ide {tool} must keep original method");
    }
}

/// P4 / T7 Normal: old two-slot tool surfaces stay distinguishable.
#[test]
fn p4_old_two_slot_surfaces_remain_distinguishable() {
    let todo = names_of(&tools_list_for_slot("todo_task"));
    let ide = names_of(&tools_list_for_slot("cursor_ide"));
    assert!(!todo.is_empty());
    assert!(!ide.is_empty());
    assert_ne!(
        todo, ide,
        "todo_task and cursor_ide tool surfaces must remain distinguishable"
    );
}

/// P4 / T7 Boundary: get_notes_selection and any notes-only tools stay off old slots.
#[test]
fn p4_notes_proprietary_tools_do_not_appear_on_old_slots() {
    let proprietary = notes_proprietary_tool_names();
    assert!(
        proprietary.contains(NOTES_SELECTION_TOOL),
        "get_notes_selection must be treated as notes-proprietary"
    );
    let locked: BTreeSet<_> = super::NOTES_SLOT_ONLY_TOOLS.iter().copied().collect();
    let observed: BTreeSet<_> = proprietary.iter().map(String::as_str).collect();
    assert_eq!(
        observed, locked,
        "notes-proprietary set must match NOTES_SLOT_ONLY_TOOLS"
    );

    for slot in ["todo_task", "cursor_ide"] {
        let names = names_of(&tools_list_for_slot(slot));
        assert!(
            !names.contains(NOTES_SELECTION_TOOL),
            "get_notes_selection must not appear on {slot}"
        );
        for tool in &proprietary {
            assert!(
                !names.contains(tool),
                "notes-proprietary tool {tool} must not appear on {slot}"
            );
        }
    }
}

/// P4 / T7 Boundary: notes tool surface is distinguishable from both old slots.
#[test]
fn p4_notes_surface_distinguishable_from_old_two_slots() {
    let notes = names_of(&tools_list_for_slot("notes"));
    let todo = names_of(&tools_list_for_slot("todo_task"));
    let ide = names_of(&tools_list_for_slot("cursor_ide"));
    assert_ne!(notes, todo, "notes must be distinguishable from todo_task");
    assert_ne!(notes, ide, "notes must be distinguishable from cursor_ide");
}

/// P4 / T7 Exception: unregistered slots still hard-reject after notes is registered.
#[test]
fn p4_unregistered_slot_still_hard_rejects() {
    assert!(
        build_slot_tool_table("notes").is_some(),
        "notes must be registered so hard-reject stays a distinct path"
    );
    assert!(build_slot_tool_table("__unknown__").is_none());
    assert!(build_slot_tool_table("").is_none());
    assert!(tools_list_for_slot("__unknown__").is_empty());

    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"p4-unknown","version":"0.0.1"}}}"#;
    let (status, body, session) = http_post_json(
        &format!("http://127.0.0.1:{}/mcp/__unknown__", local.port()),
        init,
    );
    assert_eq!(
        status, 404,
        "unregistered slot must still HTTP hard-reject, body={body}"
    );
    let json: serde_json::Value = serde_json::from_str(&body).expect("reject JSON");
    assert_eq!(
        json.get("error").and_then(|v| v.as_str()),
        Some("unknown_scene_slot"),
        "reject body={body}"
    );
    assert!(
        session.is_none(),
        "unregistered slot must still not establish an MCP session"
    );

    stop_embedded_mcp_runtime(handle).expect("stop");
}

