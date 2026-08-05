use std::collections::BTreeSet;
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
        other => panic!("unexpected tool in fixture: {other}"),
    }
}

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

/// Exception: MCP bind failure returns Err (fail-closed); must not pretend dual-listen succeeded.
#[test]
fn start_embedded_mcp_runtime_fails_closed_when_port_busy() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let _holder = TcpListener::bind(bind_addr).expect("occupy port");

    let result = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr });
    assert!(
        result.is_err(),
        "busy port must fail closed (no silent dual-listen success)"
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
