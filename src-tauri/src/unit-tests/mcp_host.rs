use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use super::*;
use crate::main_host;
use crate::mcp_host::catalog::groups::notes::{
    create_note_from_content, create_note_from_source, note_path_invoke,
};
use crate::services::mcp_oauth::{
    issue_for_device, issue_for_slot, revoke_for_device, revoke_for_slot, OAuthError, Slot,
    TicketHandle,
};
use crate::test_support::TestSandbox;
use rmcp::{
    ServiceExt,
    model::{
        CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation,
    },
    transport::{
        StreamableHttpClientTransport,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
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
    http_post_json_auth(url, body, None, None)
}

fn http_post_json_auth(
    url: &str,
    body: &str,
    authorization: Option<&str>,
    session_id: Option<&str>,
) -> (u16, String, Option<String>) {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("http client");
    let mut request = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .body(body.to_string());
    if let Some(authorization) = authorization {
        request = request.header("Authorization", authorization);
    }
    if let Some(session_id) = session_id {
        request = request.header("mcp-session-id", session_id);
    }
    let response = request.send().expect("POST");
    let status = response.status().as_u16();
    let session = response
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body = response.text().expect("body");
    (status, body, session)
}

fn issue_live_ticket(slot: &str) -> TicketHandle {
    issue_for_slot(Slot::parse(slot).expect("registered slot")).expect("issue live ticket")
}

fn bearer(ticket: &TicketHandle) -> String {
    format!("Bearer {}", ticket.as_str())
}

fn authed_transport_config(
    url: String,
    ticket: &TicketHandle,
) -> StreamableHttpClientTransportConfig {
    StreamableHttpClientTransportConfig::with_uri(url).auth_header(ticket.as_str().to_string())
}

async fn run_initialize_and_list_tools_authed(
    port: u16,
    slot: &str,
    ticket: &TicketHandle,
) -> Result<Vec<String>, CloseGateError> {
    let url = format!("http://127.0.0.1:{port}/mcp/{slot}");
    let transport = StreamableHttpClientTransport::from_config(authed_transport_config(url, ticket));
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
    let names: Vec<String> = tools.tools.iter().map(|t| t.name.to_string()).collect();
    let _ = client.cancel().await;
    Ok(names)
}

fn assert_uniform_401(status: u16, body: &str, secret: Option<&str>) {
    assert_eq!(
        status, 401,
        "registered slot must 401 without distinction, body={body}"
    );
    if let Some(secret) = secret {
        assert!(!secret.is_empty(), "ticket secret must be non-empty");
        assert!(!body.contains(secret), "401 must not write ticket secret");
    }
    let lower = body.to_ascii_lowercase();
    assert!(
        !(lower.contains("authorization:")
            && lower.contains("bearer ")
            && !lower.contains("[redacted]")),
        "401 must not write full Authorization, body={body}"
    );
    assert!(
        !body.contains("keychain_unavailable"),
        "401 must not expose keychain_unavailable"
    );
    assert!(
        !body.contains("\"rejected\""),
        "401 must not expose rejected"
    );
}

/// Notes tools from Node `buildServer()` when `includeNotes` is true.
const NOTES_TOOLS: &[&str] = &[
    "get_all_notes_catalog",
    "get_latest_digest_per_catalog",
    "get_notes_by_catalog",
    "get_note_digest_by_id",
    "get_note_content",
    "create_note",
    "delete_note",
    "list_notes_categories",
    "create_notes_category",
    "update_notes_category",
    "delete_notes_category",
];

/// Notes tools exposed on `/mcp/cursor_ide` and `/mcp/mobile` (channel hard-gate).
const NOTES_TOOLS_NON_WORKBENCH: &[&str] = &[
    "get_all_notes_catalog",
    "get_latest_digest_per_catalog",
    "get_notes_by_catalog",
    "get_note_digest_by_id",
    "get_note_content",
    "create_note",
    "list_notes_categories",
    "create_notes_category",
    "update_notes_category",
    "delete_notes_category",
];

const KNOWLEDGE_TOOLS: &[&str] = &[
    "list_knowledge_categories",
    "list_knowledge_repos",
    "get_knowledge_content",
];

const GLOBAL_TOOLS: &[&str] = &["search_document"];

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

const WORKBENCH_SLOT: &str = "workbench";
const CURSOR_IDE_SLOT: &str = "cursor_ide";

fn workbench_expected_tool_names() -> BTreeSet<&'static str> {
    let mut names: BTreeSet<&'static str> = NOTES_TOOLS.iter().copied().collect();
    names.extend(TODO_TOOLS.iter().copied());
    names.extend(KNOWLEDGE_TOOLS.iter().copied());
    names.extend(GLOBAL_TOOLS.iter().copied());
    names
}

fn cursor_ide_expected_tool_names() -> BTreeSet<&'static str> {
    let mut names: BTreeSet<&'static str> = NOTES_TOOLS_NON_WORKBENCH.iter().copied().collect();
    names.extend(TODO_TOOLS.iter().copied());
    names.extend(KNOWLEDGE_TOOLS.iter().copied());
    names.extend(GLOBAL_TOOLS.iter().copied());
    names
}

fn names_of(tools: &[ToolDescriptor]) -> BTreeSet<String> {
    tools.iter().map(|t| t.name.clone()).collect()
}

fn plant_todo_migration_gate() {
    let root = crate::config::paths::todo_tasks_dir().expect("todo dir");
    fs::create_dir_all(&root).expect("todo root");
    fs::write(
        root.join(crate::services::todo_task::MIGRATION_GATE_FILE),
        b"ok\n",
    )
    .expect("plant migration gate");
}

fn plant_empty_notes_index() {
    let notes = crate::config::paths::notes_root().expect("notes dir");
    fs::create_dir_all(&notes).expect("notes root");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("plant empty notes index");
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
        "dropping McpRuntimeHandle must release the listen socket: {rebind:?}"
    );
}

/// Normal: Streamable HTTP service is nested under `/mcp/<scene_slot>` and reachable.
#[test]
fn streamable_http_nested_under_mcp_scene_slot_is_reachable() {
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().expect("parse bind addr");
    let nest_path = "/mcp/workbench";
    let handle = start_mcp_listener(bind_addr).expect("start nested MCP scaffold");
    let local = handle.local_addr();
    let ticket = issue_live_ticket(WORKBENCH_SLOT);
    let auth = bearer(&ticket);

    // Minimal HTTP probe: a POST to the nested MCP path must get an HTTP response
    // (status line), proving the nest mount is live — not connection-refused / bare TCP.
    let mut stream = TcpStream::connect_timeout(&local, Duration::from_millis(500))
        .expect("connect to MCP listener");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("read timeout");
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t1-scaffold","version":"0.0.1"}}}"#;
    let req = format!(
        "POST {nest_path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nAuthorization: {auth}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
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
    let http_handle = main_host::start(repo_root, http_port).expect("start sidecar");
    thread::sleep(Duration::from_millis(50));

    let mcp_port = ephemeral_port();
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{mcp_port}").parse().expect("mcp addr"),
    })
    .expect("start MCP runtime");
    let mcp_local = mcp_handle.local_addr();

    let (sidecar_status, sidecar_body) =
        http_get(&format!("http://127.0.0.1:{http_port}/api/unknown"));
    assert_eq!(
        sidecar_status, 404,
        "Main Host must answer HTTP after start, body={sidecar_body}"
    );

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
    main_host::stop(http_handle);
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
    let http_handle = main_host::start(repo_root, http_port).expect("start sidecar");
    thread::sleep(Duration::from_millis(50));

    let mcp_port = ephemeral_port();
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{mcp_port}").parse().expect("mcp addr"),
    })
    .expect("start MCP");

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP only");

    let (status, body) = http_get(&format!("http://127.0.0.1:{http_port}/api/unknown"));
    assert_eq!(
        status, 404,
        "Main Host tiny_http must keep serving after MCP stop, body={body}"
    );

    main_host::stop(http_handle);
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

/// Normal: `workbench` routing table = notes ∪ todo ∪ knowledge.
#[test]
fn build_slot_tool_table_workbench_is_notes_todo() {
    let table = build_slot_tool_table(WORKBENCH_SLOT).expect("workbench registered");
    assert_eq!(table.scene_slot, WORKBENCH_SLOT);
    assert!(!table.tools.is_empty());

    let names: BTreeSet<_> = table.tools.iter().map(|t| t.name.as_str()).collect();
    let expected = workbench_expected_tool_names();
    assert_eq!(names, expected, "workbench tools must be notes ∪ todo ∪ knowledge");
    assert!(
        !names.contains("get_notes_selection"),
        "workbench must not hang get_notes_selection"
    );
    assert!(
        table.tools.iter().all(|r| r.invoke as usize != 0),
        "every workbench tool must bind an in-process Services invoke"
    );
}

/// Normal: `cursor_ide` routing table = notes ∪ todo ∪ knowledge (SCENE_SLOT_API both true).
#[test]
fn build_slot_tool_table_cursor_ide_matches_node_allowlist() {
    let table = build_slot_tool_table("cursor_ide").expect("cursor_ide registered");
    assert_eq!(table.scene_slot, "cursor_ide");
    assert!(!table.tools.is_empty());

    let names: BTreeSet<_> = table.tools.iter().map(|t| t.name.as_str()).collect();
    let expected = workbench_expected_tool_names();
    assert_eq!(
        names, expected,
        "cursor_ide slot catalog is notes ∪ todo ∪ knowledge"
    );
    assert!(
        table.tools.iter().all(|r| r.invoke as usize != 0),
        "every cursor_ide tool must bind an in-process Services invoke"
    );
}

/// Boundary: workbench and cursor_ide both expose notes ∪ todo ∪ knowledge; notes-selection is gone.
#[test]
fn tools_list_workbench_and_cursor_ide_are_notes_todo() {
    let wb_names = names_of(&tools_list_for_slot(WORKBENCH_SLOT));
    let ide_names = names_of(&tools_list_for_slot(CURSOR_IDE_SLOT));

    assert!(!wb_names.is_empty());
    assert!(!ide_names.is_empty());
    assert!(
        !wb_names.contains("get_notes_selection"),
        "tools_list_for_slot(workbench) must not include get_notes_selection"
    );
    assert!(
        !ide_names.contains("get_notes_selection"),
        "cursor_ide must not include get_notes_selection"
    );
    for tool in NOTES_TOOLS {
        assert!(
            ide_names.contains(*tool),
            "cursor_ide must include notes tool {tool}"
        );
        assert!(
            wb_names.contains(*tool),
            "workbench must include notes tool {tool}"
        );
    }
    for todo in TODO_TOOLS {
        assert!(
            ide_names.contains(*todo),
            "cursor_ide must keep todo tool {todo}"
        );
        assert!(
            wb_names.contains(*todo),
            "workbench must include todo tool {todo}"
        );
    }
    for tool in KNOWLEDGE_TOOLS {
        assert!(
            ide_names.contains(*tool),
            "cursor_ide must include knowledge tool {tool}"
        );
        assert!(
            wb_names.contains(*tool),
            "workbench must include knowledge tool {tool}"
        );
    }
    for tool in GLOBAL_TOOLS {
        assert!(
            ide_names.contains(*tool),
            "cursor_ide must include global tool {tool}"
        );
        assert!(
            wb_names.contains(*tool),
            "workbench must include global tool {tool}"
        );
    }
    assert_eq!(
        wb_names, ide_names,
        "workbench and cursor_ide now share notes ∪ todo ∪ knowledge ∪ global"
    );
}

#[test]
fn mcp_tools_list_publishes_descriptions_schemas_and_mutation_hints() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");

    let list_tools = |slot: &str, ticket: TicketHandle| {
        let slot = slot.to_string();
        let port = local.port();
        async move {
            let url = format!("http://127.0.0.1:{port}/mcp/{slot}");
            let transport = StreamableHttpClientTransport::from_config(authed_transport_config(url, &ticket));
            let client = ClientInfo::new(
                ClientCapabilities::default(),
                Implementation::new("tool-contract-test", "0.1.0"),
            )
            .serve(transport)
            .await
            .expect("initialize");
            let tools = client.list_tools(Default::default()).await.expect("tools/list");
            let _ = client.cancel().await;
            tools.tools
        }
    };

    let workbench_ticket = issue_live_ticket(WORKBENCH_SLOT);
    let workbench_tools = rt.block_on(list_tools(WORKBENCH_SLOT, workbench_ticket.clone()));
    assert_eq!(
        workbench_tools.len(),
        workbench_expected_tool_names().len(),
        "Workbench MCP tool count"
    );
    for tool in &workbench_tools {
        assert!(
            tool.description.as_deref().is_some_and(|d| !d.trim().is_empty()),
            "{} needs a model-facing description",
            tool.name
        );
        assert_eq!(
            tool.input_schema["type"], "object",
            "{} needs an object input schema",
            tool.name
        );
        assert!(
            tool.annotations.is_some(),
            "{} needs read/write annotations",
            tool.name
        );
    }
    let create = workbench_tools
        .iter()
        .find(|tool| tool.name == "create_todo_task")
        .expect("create_todo_task");
    assert!(
        create.description.as_deref().is_some_and(|d| !d.trim().is_empty()),
        "model-facing Todo tool needs a description"
    );
    assert_eq!(
        create.input_schema["properties"]["title"]["type"], "string",
        "create_todo_task title must be declared"
    );
    assert_eq!(
        create.input_schema["properties"]["todo_md"]["type"], "string",
        "create_todo_task todo_md must be declared"
    );
    assert!(
        create.input_schema["required"]
            .as_array()
            .is_some_and(|required| required.iter().any(|v| v == "title")),
        "create_todo_task title must be required"
    );
    assert!(
        create.input_schema["required"]
            .as_array()
            .is_some_and(|required| required.iter().any(|v| v == "todo_md")),
        "create_todo_task todo_md must be required"
    );
    assert_eq!(
        create
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.read_only_hint),
        Some(false),
        "Todo creation must be identified as mutating"
    );
    assert!(
        create.input_schema["properties"].get("sub_titles").is_none(),
        "create_todo_task must not advertise sub_titles"
    );

    let add_sub = workbench_tools
        .iter()
        .find(|tool| tool.name == "add_todo_sub")
        .expect("add_todo_sub");
    assert!(
        add_sub.input_schema["required"]
            .as_array()
            .is_some_and(|required| required.iter().any(|v| v == "content")),
        "add_todo_sub content must be required"
    );

    let notes_tools = rt.block_on(list_tools(WORKBENCH_SLOT, workbench_ticket));
    assert_eq!(
        notes_tools.len(),
        workbench_expected_tool_names().len(),
        "Workbench MCP tool count is notes ∪ todo ∪ knowledge ∪ global"
    );
    for tool in &notes_tools {
        assert!(
            tool.description.as_deref().is_some_and(|d| !d.trim().is_empty()),
            "{} needs a model-facing description",
            tool.name
        );
        assert_eq!(
            tool.input_schema["type"], "object",
            "{} needs an object input schema",
            tool.name
        );
        assert!(
            tool.annotations.is_some(),
            "{} needs read/write annotations",
            tool.name
        );
    }
    let catalog = notes_tools
        .iter()
        .find(|tool| tool.name == "get_all_notes_catalog")
        .expect("get_all_notes_catalog");
    assert!(
        catalog
            .input_schema
            .get("required")
            .and_then(|v| v.as_array())
            .map(|required| required.is_empty())
            .unwrap_or(true),
        "catalog list must not require mode"
    );
    assert!(
        notes_tools.iter().any(|tool| tool.name == "get_note_content"),
        "raw read must be hung"
    );
    assert!(
        notes_tools.iter().any(|tool| tool.name == "search_document"),
        "document search must be hung"
    );
    assert!(
        !notes_tools
            .iter()
            .any(|tool| tool.name.contains("notes_selection")),
        "notes-selection MCP tools are gone"
    );

    stop_embedded_mcp_runtime(handle).expect("stop MCP");
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
    for slot in [WORKBENCH_SLOT, CURSOR_IDE_SLOT] {
        let ticket = issue_live_ticket(slot);
        let auth = bearer(&ticket);
        let (status, body, _) = http_post_json_auth(
            &format!("http://127.0.0.1:{}/mcp/{slot}", local.port()),
            init,
            Some(&auth),
            None,
        );
        assert_ne!(
            status, 404,
            "registered slot {slot} must not hard-reject, body={body}"
        );
        assert_ne!(
            status, 401,
            "registered slot {slot} holding a Live ticket must not 401, body={body}"
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

/// Normal: tools/call reaches L4 Services in-process (no :8765 hop).
#[test]
fn representative_tools_call_per_registered_slot_hits_services() {
    let _sandbox = TestSandbox::new();
    plant_todo_migration_gate();
    plant_empty_notes_index();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let cats = rt
        .block_on(proxy_tool_call(
            WORKBENCH_SLOT,
            WORKBENCH_SLOT,
            "list_todo_categories",
            serde_json::json!({}),
        ))
        .expect("proxy")
        .expect("todo categories succeed");
    let cats_json: serde_json::Value =
        serde_json::from_str(mcp_text_ok(&cats)).expect("categories JSON");
    assert!(
        cats_json.get("categories").is_some() || cats_json.is_object() || cats_json.is_array(),
        "list_todo_categories must return Services wire, got {}",
        mcp_text_ok(&cats)
    );

    let catalogs = rt
        .block_on(proxy_tool_call(
            CURSOR_IDE_SLOT,
            CURSOR_IDE_SLOT,
            "get_all_notes_catalog",
            serde_json::json!({}),
        ))
        .expect("proxy")
        .expect("notes catalogs succeed");
    let catalog_json: serde_json::Value =
        serde_json::from_str(mcp_text_ok(&catalogs)).expect("catalogs JSON");
    assert!(
        catalog_json.get("items").is_some(),
        "get_all_notes_catalog must return items, got {}",
        mcp_text_ok(&catalogs)
    );
}

/// Regression: MCP dispatch must isolate a synchronous search from the
/// async MCP worker; otherwise a blocking client panics the worker.
#[test]
fn proxy_dispatches_search_document_on_blocking_worker() {
    let _sandbox = TestSandbox::new();
    plant_empty_notes_index();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");

    let result = rt
        .block_on(proxy_tool_call(
            WORKBENCH_SLOT,
            WORKBENCH_SLOT,
            "search_document",
            serde_json::json!({ "q": "regression" }),
        ))
        .expect("blocking worker must return a mapped tool result");
    // Sandbox has no keyword-index.sqlite → Services answer 503 not_indexed,
    // which must surface as a mapped tool error, not an MCP worker panic.
    match result {
        Ok(_) => {}
        Err(err) => assert!(
            mcp_text_err(&err).contains("not_indexed"),
            "unexpected mapped error: {err:?}"
        ),
    }
}

/// Exception: Services `_status` maps to the same MCP error wire as the old HTTP mapper.
#[test]
fn proxy_tool_call_maps_service_error_like_node() {
    let err = map_service_value_to_mcp(serde_json::json!({
        "error": "boom",
        "_status": 500
    }))
    .expect_err("500 → tool error");
    assert_eq!(mcp_text_err(&err), r#"HTTP 500: {"error":"boom"}"#);
}

/// Catalog invoke reaches L4; tool dispatch must not hop to Main Host :8765.
#[test]
fn tool_dispatch_is_in_process_services() {
    let proxy = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp_host/server/proxy.rs"
    ));
    assert!(
        !proxy.contains("8765") && !proxy.contains("sidecar_base_url") && !proxy.contains("reqwest"),
        "proxy.rs must dispatch in-process, not HTTP to Main Host"
    );
    assert!(
        !adapter_source().contains("DEFAULT_SIDECAR_BASE_URL"),
        "MCP Host must not default a Sidecar base URL"
    );
    let catalog = crate::test_support::read_rs_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/mcp_host/catalog/groups"
    ));
    assert!(
        catalog.contains("crate::services::todo_task")
            && catalog.contains("crate::services::workbench_read")
            && catalog.contains("crate::services::notes"),
        "catalog invoke must call L4 Services"
    );
}

/// Production close-gate ports (V5 / Topic2): MCP `:9876` + Sidecar `:8765`.
const CLOSE_GATE_MCP_PORT: u16 = 9876;
const CLOSE_GATE_SIDECAR_PORT: u16 = 8765;

fn start_close_gate_dual_listen() -> (main_host::MainHostHandle, McpRuntimeHandle) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let http_handle =
        main_host::start(repo_root, CLOSE_GATE_SIDECAR_PORT).expect("start Sidecar :8765");
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
    let ticket = issue_live_ticket(WORKBENCH_SLOT);

    let dual_listen_observed = observe_dual_listen(CLOSE_GATE_MCP_PORT, CLOSE_GATE_SIDECAR_PORT)
        .expect("P1 close gate must observe dual listen");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");
    let tool_names = rt
        .block_on(run_initialize_and_list_tools_authed(
            CLOSE_GATE_MCP_PORT,
            WORKBENCH_SLOT,
            &ticket,
        ))
        .expect("P1 close gate must pass before Node spawn hard-cut");

    assert!(
        dual_listen_observed,
        "same-process :9876 + :8765 must be observable"
    );
    assert!(
        !tool_names.is_empty(),
        "tools/list must return at least one tool name"
    );
    assert!(
        tool_names.iter().any(|n| n == "list_todo_categories"),
        "workbench tools/list must include list_todo_categories, got {:?}",
        tool_names
    );

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    main_host::stop(http_handle);
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

    let ticket = issue_live_ticket(WORKBENCH_SLOT);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");
    let tool_names = rt
        .block_on(run_initialize_and_list_tools_authed(
            CLOSE_GATE_MCP_PORT,
            WORKBENCH_SLOT,
            &ticket,
        ))
        .expect("session close gate");
    assert!(
        !tool_names.is_empty(),
        "session initialize+tools/list is the close-gate proof, not /health alone"
    );

    stop_embedded_mcp_runtime(mcp_handle).expect("stop MCP");
    main_host::stop(http_handle);
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
    main_host::stop(http_handle);
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
    // Main Host fixture: independent listen on :8765 with migration gate planted.
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let todo_root = wb.join("todo_tasks");
    fs::create_dir_all(&todo_root).expect("mkdir todo_tasks");
    fs::write(todo_root.join(".migration_gate_passed"), b"ok\n").expect("plant migration gate");
    let notes = wb.join("notes");
    fs::create_dir_all(&notes).expect("notes");
    fs::write(notes.join("index.json"), br#"{"entries":{}}"#).expect("plant empty notes index");

    let http_handle =
        main_host::start(sandbox.config_dir().to_path_buf(), CLOSE_GATE_SIDECAR_PORT)
            .expect("start Sidecar :8765 independent of MCP");
    thread::sleep(Duration::from_millis(50));
    let mcp_handle = start_embedded_mcp_runtime(McpRuntimeConfig {
        bind_addr: format!("127.0.0.1:{CLOSE_GATE_MCP_PORT}")
            .parse()
            .expect("mcp addr"),
    })
    .expect("start Host MCP :9876");

    let (sidecar_status, _) = http_get(&format!(
        "http://127.0.0.1:{CLOSE_GATE_SIDECAR_PORT}/api/unknown"
    ));
    assert_eq!(
        sidecar_status, 404,
        "T10: Main Host HTTP must be independently observable on :8765"
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

    let workbench_ticket = issue_live_ticket(WORKBENCH_SLOT);
    let workbench_names = rt
        .block_on(run_initialize_and_list_tools_authed(
            CLOSE_GATE_MCP_PORT,
            WORKBENCH_SLOT,
            &workbench_ticket,
        ))
        .expect("workbench tools/list on Host :9876");
    for tool in ["list_todo_tasks", "create_todo_task"] {
        assert!(
            workbench_names.iter().any(|n| n == tool),
            "T10/V2: workbench missing {tool}; got {workbench_names:?}"
        );
    }
    for tool in NOTES_TOOLS {
        assert!(
            workbench_names.iter().any(|n| n == *tool),
            "T10/V2: workbench missing notes tool {tool}"
        );
    }

    let ide_ticket = issue_live_ticket(CURSOR_IDE_SLOT);
    let ide_names = rt
        .block_on(run_initialize_and_list_tools_authed(
            CLOSE_GATE_MCP_PORT,
            CURSOR_IDE_SLOT,
            &ide_ticket,
        ))
        .expect("cursor_ide tools/list on Host :9876");
    for tool in NOTES_TOOLS_NON_WORKBENCH {
        assert!(
            ide_names.iter().any(|n| n == *tool),
            "T10/V2: cursor_ide missing {tool}; got {ide_names:?}"
        );
    }
    assert!(
        !ide_names.iter().any(|n| n == "delete_note"),
        "T10/V2: cursor_ide must not expose workbench-only delete_note"
    );
    assert!(
        !ide_names.iter().any(|n| n == "get_notes_selection"),
        "T10/V2: cursor_ide must not gain notes-selection tools"
    );
    assert!(
        !workbench_names.iter().any(|n| n == "get_notes_selection"),
        "T10/V2: workbench must not expose get_notes_selection"
    );
    let wb_set: BTreeSet<_> = workbench_names.iter().map(String::as_str).collect();
    let ide_set: BTreeSet<_> = ide_names.iter().map(String::as_str).collect();
    assert_eq!(
        wb_set,
        workbench_expected_tool_names(),
        "T10/V2: workbench tools/list"
    );
    assert_eq!(
        ide_set,
        cursor_ide_expected_tool_names(),
        "T10/V2: cursor_ide tools/list"
    );

    async fn list_and_call(
        port: u16,
        slot: &str,
        ticket: &TicketHandle,
        tool: &str,
        args: serde_json::Value,
    ) -> Result<(Vec<String>, bool, String), String> {
        let url = format!("http://127.0.0.1:{port}/mcp/{slot}");
        let transport = StreamableHttpClientTransport::from_config(authed_transport_config(url, ticket));
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
            WORKBENCH_SLOT,
            &workbench_ticket,
            "list_todo_tasks",
            serde_json::json!({}),
        ))
        .expect("workbench list_todo_tasks");
    assert!(
        !todo_err,
        "T10/V2: workbench representative tools/call must succeed via Host→Sidecar; got {todo_text}"
    );

    let (_, ide_err, ide_text) = rt
        .block_on(list_and_call(
            CLOSE_GATE_MCP_PORT,
            "cursor_ide",
            &ide_ticket,
            "get_all_notes_catalog",
            serde_json::json!({}),
        ))
        .expect("cursor_ide get_all_notes_catalog");
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
    main_host::stop(http_handle);
    drop(sandbox);
}

fn adapter_source() -> &'static str {
    static SRC: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SRC.get_or_init(|| {
        crate::test_support::read_rs_dir(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/mcp_host"
        ))
    })
    .as_str()
}

fn source_fn_after<'a>(src: &'a str, fn_name: &str) -> &'a str {
    src.split(&format!("fn {fn_name}"))
        .nth(1)
        .unwrap_or_else(|| panic!("{fn_name} must exist"))
}

fn assert_old_app_slots_unregistered() {
    for slot in ["notes", "todo_task"] {
        assert!(
            build_slot_tool_table(slot).is_none(),
            "old App slot {slot} must not have a tool table"
        );
        assert!(
            tools_list_for_slot(slot).is_empty(),
            "tools_list_for_slot({slot}) must be empty"
        );
        assert!(
            !super::is_registered_scene_slot(slot),
            "is_registered_scene_slot({slot}) must be false"
        );
    }
}

/// Normal: workbench tools/list = notes ∪ todo ∪ knowledge.
#[test]
fn tools_list_for_workbench_is_notes_todo() {
    let names = names_of(&tools_list_for_slot(WORKBENCH_SLOT));
    let expected: BTreeSet<_> = workbench_expected_tool_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(names, expected, "workbench tools/list must be notes ∪ todo ∪ knowledge");
    assert!(!names.contains("get_notes_selection"));
}

/// Normal: REGISTERED_SCENE_SLOTS is only workbench + cursor_ide.
#[test]
fn registered_scene_slots_are_only_workbench_and_cursor_ide() {
    assert_eq!(
        super::REGISTERED_SCENE_SLOTS,
        &[WORKBENCH_SLOT, CURSOR_IDE_SLOT]
    );
    assert!(!super::REGISTERED_SCENE_SLOTS.contains(&"notes"));
    assert!(!super::REGISTERED_SCENE_SLOTS.contains(&"todo_task"));
    assert!(!super::REGISTERED_SCENE_SLOTS.contains(&"mobile"));
    assert!(super::is_registered_scene_slot(WORKBENCH_SLOT));
    assert!(super::is_registered_scene_slot(CURSOR_IDE_SLOT));
}

/// Boundary: get_notes_selection is not hung on workbench.
#[test]
fn get_notes_selection_is_not_hung_on_workbench() {
    let table = build_slot_tool_table(WORKBENCH_SLOT).expect("workbench registered");
    assert!(
        table
            .tools
            .iter()
            .all(|r| r.name != "get_notes_selection"),
        "build_slot_tool_table must not attach notes-selection"
    );

    let src = adapter_source();
    assert!(
        !src.contains("get_notes_selection") && !src.contains("NOTES_SLOT_ONLY_TOOLS"),
        "mcp_host must not mention notes-selection tools"
    );
}

/// Boundary: old App slots have no tool table.
#[test]
fn old_app_slots_have_no_tool_table() {
    assert_old_app_slots_unregistered();
}

/// Exception: workbench exposes no MCP write-selection tool.
#[test]
fn workbench_slot_has_no_mcp_write_selection_tool() {
    let table = build_slot_tool_table(WORKBENCH_SLOT).expect("workbench registered");
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
    }
}

/// Normal: /mcp/workbench is mounted and tools/list matches the union surface.
#[test]
fn workbench_scene_slot_initialize_lists_union_plus_selection() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let ticket = issue_live_ticket(WORKBENCH_SLOT);
    let auth = bearer(&ticket);
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t3-workbench","version":"0.0.1"}}}"#;
    let (status, body, _) = http_post_json_auth(
        &format!("http://127.0.0.1:{}/mcp/workbench", local.port()),
        init,
        Some(&auth),
        None,
    );
    assert_ne!(
        status, 404,
        "registered workbench slot must not hard-reject, body={body}"
    );
    assert_ne!(
        status, 401,
        "registered workbench slot holding a Live ticket must not 401, body={body}"
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio");
    let names = rt
        .block_on(run_initialize_and_list_tools_authed(
            local.port(),
            WORKBENCH_SLOT,
            &ticket,
        ))
        .expect("workbench tools/list");
    let got: BTreeSet<_> = names.iter().map(String::as_str).collect();
    let expected = workbench_expected_tool_names();
    assert_eq!(got, expected, "HTTP tools/list for /mcp/workbench");

    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Exception: /mcp/notes and /mcp/todo_task hard-reject (not in registered table).
#[test]
fn old_app_slots_http_hard_reject_without_mcp_session() {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"t3-old-slots","version":"0.0.1"}}}"#;
    for slot in ["notes", "todo_task"] {
        let (status, body, session) = http_post_json(
            &format!("http://127.0.0.1:{}/mcp/{slot}", local.port()),
            init,
        );
        assert_eq!(
            status, 404,
            "old App slot {slot} must HTTP hard-reject, body={body}"
        );
        let json: serde_json::Value = serde_json::from_str(&body).expect("reject JSON");
        assert_eq!(
            json.get("error").and_then(|v| v.as_str()),
            Some("unknown_scene_slot"),
            "reject body={body}"
        );
        assert!(
            session.is_none(),
            "must not establish MCP session for old slot {slot}"
        );
    }

    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// P4: cursor_ide remains notes four-pack + all todo, original `/api` routes; not App-global.
#[test]
fn p4_cursor_ide_surface_unchanged_notes_plus_todo_original_api() {
    let table = build_slot_tool_table(CURSOR_IDE_SLOT).expect("cursor_ide registered");
    assert_eq!(table.scene_slot, CURSOR_IDE_SLOT);
    assert!(!table.tools.is_empty());

    let names = names_of(&tools_list_for_slot(CURSOR_IDE_SLOT));
    // Slot catalog still includes workbench-only tools; channel overlay strips them at listen time.
    let expected: BTreeSet<_> = workbench_expected_tool_names()
        .into_iter()
        .map(str::to_string)
        .collect();
    assert_eq!(
        names, expected,
        "cursor_ide slot catalog must stay notes ∪ todo ∪ knowledge (channel hard-gates are separate)"
    );
    assert!(
        !names.contains("get_notes_selection"),
        "cursor_ide must not expose get_notes_selection"
    );
    assert!(
        table.tools.iter().all(|r| r.invoke as usize != 0),
        "cursor_ide tools must bind in-process Services invokes"
    );
}

/// P4: neither slot hangs notes-selection tools.
#[test]
fn p4_notes_selection_tools_are_gone_from_both_slots() {
    let wb = names_of(&tools_list_for_slot(WORKBENCH_SLOT));
    let ide = names_of(&tools_list_for_slot(CURSOR_IDE_SLOT));
    assert!(!wb.contains("get_notes_selection"));
    assert!(!ide.contains("get_notes_selection"));
    assert_eq!(wb, ide);
}

/// P4: unknown / old App slots still hard-reject after workbench is registered.
#[test]
fn p4_unregistered_and_old_app_slots_still_hard_reject() {
    assert!(build_slot_tool_table(WORKBENCH_SLOT).is_some());
    assert_old_app_slots_unregistered();
    assert!(build_slot_tool_table("__unknown__").is_none());
    assert!(build_slot_tool_table("").is_none());
    assert!(tools_list_for_slot("__unknown__").is_empty());

    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    let handle = start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start");
    let local = handle.local_addr();

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"p4-unknown","version":"0.0.1"}}}"#;
    for slot in ["__unknown__", "notes", "todo_task"] {
        let (status, body, session) = http_post_json(
            &format!("http://127.0.0.1:{}/mcp/{slot}", local.port()),
            init,
        );
        assert_eq!(
            status, 404,
            "unregistered slot {slot} must still HTTP hard-reject, body={body}"
        );
        let json: serde_json::Value = serde_json::from_str(&body).expect("reject JSON");
        assert_eq!(
            json.get("error").and_then(|v| v.as_str()),
            Some("unknown_scene_slot"),
            "reject body={body}"
        );
        assert!(
            session.is_none(),
            "unregistered slot {slot} must still not establish an MCP session"
        );
    }

    stop_embedded_mcp_runtime(handle).expect("stop");
}

fn start_ephemeral_mcp() -> McpRuntimeHandle {
    let port = ephemeral_port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start")
}

fn initialize_body(name: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-03-26","capabilities":{{}},"clientInfo":{{"name":"{name}","version":"0.0.1"}}}}}}"#
    )
}

/// Normal: registered workbench slot holding that slot's Live ticket enters Streamable HTTP.
#[test]
fn registered_workbench_live_ticket_enters_streamable_http() {
    let handle = start_ephemeral_mcp();
    let ticket = issue_live_ticket(WORKBENCH_SLOT);
    let auth = bearer(&ticket);
    let (status, body, session) = http_post_json_auth(
        &format!("http://127.0.0.1:{}/mcp/workbench", handle.local_addr().port()),
        &initialize_body("t2-workbench-live"),
        Some(&auth),
        None,
    );
    assert_ne!(status, 401, "Live workbench ticket must enter StreamableHttpService, body={body}");
    assert_ne!(status, 404, "registered workbench must not 404, body={body}");
    let _ = session;
    let names = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio")
        .block_on(run_initialize_and_list_tools_authed(
            handle.local_addr().port(),
            WORKBENCH_SLOT,
            &ticket,
        ))
        .expect("tools/list with Live ticket");
    assert!(
        names.iter().any(|n| n == "list_todo_categories"),
        "workbench tools/list must remain available, got {names:?}"
    );
    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Normal: registered cursor_ide slot holding that slot's Live ticket is admitted.
#[test]
fn registered_cursor_ide_live_ticket_enters_streamable_http() {
    let handle = start_ephemeral_mcp();
    let ticket = issue_live_ticket(CURSOR_IDE_SLOT);
    let names = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio")
        .block_on(run_initialize_and_list_tools_authed(
            handle.local_addr().port(),
            CURSOR_IDE_SLOT,
            &ticket,
        ))
        .expect("cursor_ide tools/list with Live ticket");
    assert!(
        names.iter().any(|n| n == "get_all_notes_catalog"),
        "cursor_ide tools/list must remain available, got {names:?}"
    );
    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Normal: GET /health stays exempt, does not consult the ledger, and keeps ok + non-empty mcp.
#[test]
fn get_health_is_exempt_and_does_not_consult_ledger() {
    let _ = revoke_for_slot(Slot::Workbench);
    let _ = revoke_for_slot(Slot::CursorIde);
    let handle = start_ephemeral_mcp();
    let (status, body) = http_get(&format!(
        "http://127.0.0.1:{}/health",
        handle.local_addr().port()
    ));
    assert_eq!(status, 200, "health must stay 200 without a ticket, body={body}");
    let json: serde_json::Value = serde_json::from_str(&body).expect("health JSON");
    assert_eq!(json.get("ok"), Some(&serde_json::Value::Bool(true)));
    let mcp = json.get("mcp").and_then(|v| v.as_str()).unwrap_or("");
    assert!(!mcp.trim().is_empty(), "health mcp field must stay non-empty, got {json}");

    let health_fn = source_fn_after(adapter_source(), "health_json")
        .split("async fn bare_mcp_reject")
        .next()
        .expect("health_json body");
    assert!(
        !health_fn.contains("verify_for_slot") && !health_fn.contains("ledger_record"),
        "GET /health must not verify or read the ledger"
    );
    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Normal: adapter only consumes verify_for_slot; it never issues tickets.
#[test]
fn adapter_only_calls_verify_for_slot_and_never_issues() {
    let src = adapter_source();
    assert!(
        src.contains("verify_for_slot"),
        "adapter must call verify_for_slot"
    );
    assert!(
        !src.contains("issue_for_slot"),
        "adapter must not issue tickets"
    );
    assert!(
        !src.contains("rotate_for_slot"),
        "adapter must not rotate tickets"
    );
}

/// Boundary: unknown slot and bare /mcp stay 404 without ledger lookup, session, or verify.
#[test]
fn unknown_and_bare_mcp_stay_404_without_ledger_or_verify() {
    let ticket = issue_live_ticket(WORKBENCH_SLOT);
    let auth = bearer(&ticket);
    let handle = start_ephemeral_mcp();
    let port = handle.local_addr().port();
    let init = initialize_body("t2-unknown-bare");

    for (url, expect_error) in [
        (format!("http://127.0.0.1:{port}/mcp/__unknown__"), "unknown_scene_slot"),
        (format!("http://127.0.0.1:{port}/mcp"), "scene_slot_required"),
    ] {
        let (status, body, session) = http_post_json_auth(&url, &init, Some(&auth), None);
        assert_eq!(status, 404, "must stay 404 without entering verify, body={body}");
        let json: serde_json::Value = serde_json::from_str(&body).expect("reject JSON");
        assert_eq!(
            json.get("error").and_then(|v| v.as_str()),
            Some(expect_error),
            "reject body={body}"
        );
        assert!(session.is_none(), "must not create an MCP session, url={url}");
    }

    let src = adapter_source();
    for (fn_name, stop) in [
        ("bare_mcp_reject", "async fn unknown_scene_slot_reject"),
        ("unknown_scene_slot_reject", "fn mount_slot_service"),
    ] {
        let body = source_fn_after(src, fn_name)
            .split(stop)
            .next()
            .unwrap_or_else(|| panic!("{fn_name} body"));
        assert!(
            !body.contains("verify_for_slot") && !body.contains("ledger_record"),
            "{fn_name} must not consult the ledger"
        );
    }
    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Boundary: every registered-slot HTTP request must carry Authorization; a session cannot replace the ticket.
#[test]
fn registered_slot_session_cannot_replace_ticket() {
    let handle = start_ephemeral_mcp();
    let ticket = issue_live_ticket(WORKBENCH_SLOT);
    let auth = bearer(&ticket);
    let url = format!(
        "http://127.0.0.1:{}/mcp/workbench",
        handle.local_addr().port()
    );
    let (status, body, _) = http_post_json_auth(
        &url,
        &initialize_body("t2-session-not-ticket"),
        Some(&auth),
        None,
    );
    assert_ne!(status, 401, "first authed request must pass, body={body}");

    let (status, body, _) = http_post_json_auth(
        &url,
        &initialize_body("t2-session-only"),
        None,
        Some("session-is-not-a-ticket"),
    );
    assert_uniform_401(status, &body, None);
    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Boundary: verify is cut before StreamableHttpService — no session without a ticket.
#[test]
fn verify_is_cut_before_streamable_http_service() {
    let src = adapter_source();
    assert!(
        src.contains("verify_for_slot"),
        "adapter must call verify_for_slot on registered slots"
    );
    let mount = source_fn_after(src, "mount_slot_service")
        .split("fn build_router")
        .next()
        .expect("mount_slot_service body");
    assert!(
        mount.contains("nest_service") || mount.contains("StreamableHttpService"),
        "registered slots still nest StreamableHttpService after the door"
    );

    let handle = start_ephemeral_mcp();
    let (status, body, session) = http_post_json(
        &format!(
            "http://127.0.0.1:{}/mcp/workbench",
            handle.local_addr().port()
        ),
        &initialize_body("t2-before-nest"),
    );
    assert_uniform_401(status, &body, None);
    assert!(
        session.is_none(),
        "verify must reject before StreamableHttpService mints a session"
    );
    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Exception: missing, unknown, cross-slot, and revoked tickets are all 401.
#[test]
fn registered_slot_missing_unknown_mismatch_revoked_are_uniform_401() {
    let handle = start_ephemeral_mcp();
    let port = handle.local_addr().port();
    let workbench_url = format!("http://127.0.0.1:{port}/mcp/workbench");
    let ide_url = format!("http://127.0.0.1:{port}/mcp/cursor_ide");
    let init = initialize_body("t2-reject-cases");

    let (status, body, session) = http_post_json(&workbench_url, &init);
    assert_uniform_401(status, &body, None);
    assert!(session.is_none(), "no-ticket request must not mint a session");

    let missing = TicketHandle::from_secret("no-such-ticket");
    let missing_auth = bearer(&missing);
    let (status, body, _) =
        http_post_json_auth(&workbench_url, &init, Some(&missing_auth), None);
    assert_uniform_401(status, &body, Some(missing.as_str()));

    let workbench = issue_live_ticket(WORKBENCH_SLOT);
    let workbench_auth = bearer(&workbench);
    let (status, body, _) = http_post_json_auth(&ide_url, &init, Some(&workbench_auth), None);
    assert_uniform_401(status, &body, Some(workbench.as_str()));

    let live = issue_live_ticket(CURSOR_IDE_SLOT);
    revoke_for_slot(Slot::CursorIde).expect("revoke");
    let revoked_auth = bearer(&live);
    let (status, body, _) = http_post_json_auth(&ide_url, &init, Some(&revoked_auth), None);
    assert_uniform_401(status, &body, Some(live.as_str()));

    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Exception: both rejected and keychain_unavailable map to 401 with no reason leak.
#[test]
fn rejected_and_keychain_unavailable_both_map_to_401() {
    let src = adapter_source();
    assert!(
        src.contains("keychain_unavailable"),
        "door must handle verify_for_slot keychain_unavailable as 401"
    );
    assert!(
        src.contains("OAuthError::rejected") || src.contains("rejected"),
        "door must handle verify_for_slot rejected as 401"
    );
    assert!(
        src.contains("UNAUTHORIZED") || src.contains("401"),
        "both verify failures must map to HTTP 401"
    );

    let handle = start_ephemeral_mcp();
    let (status, body, _) = http_post_json(
        &format!(
            "http://127.0.0.1:{}/mcp/workbench",
            handle.local_addr().port()
        ),
        &initialize_body("t2-rejected-401"),
    );
    assert_uniform_401(status, &body, None);
    stop_embedded_mcp_runtime(handle).expect("stop");
}

const MOBILE_PATH: &str = "mobile";

fn with_device_sandbox<F: FnOnce()>(test: F) {
    let _sandbox = TestSandbox::new();
    test();
}

fn issue_device_ticket(device_id: &str) -> TicketHandle {
    issue_for_device(device_id, Some("t3-phone")).expect("issue device ticket")
}

async fn list_and_call_mobile(
    port: u16,
    ticket: &TicketHandle,
    tool: &str,
    args: serde_json::Value,
) -> Result<(Vec<String>, bool, String), String> {
    let url = format!("http://127.0.0.1:{port}/mcp/{MOBILE_PATH}");
    let transport = StreamableHttpClientTransport::from_config(authed_transport_config(url, ticket));
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("t3-mobile", "0.1.0"),
    );
    let client = client_info
        .serve(transport)
        .await
        .map_err(|e| format!("initialize mobile: {e:#}"))?;
    let tools = client
        .list_tools(Default::default())
        .await
        .map_err(|e| format!("list_tools mobile: {e:#}"))?;
    let names: Vec<String> = tools.tools.iter().map(|t| t.name.to_string()).collect();
    let mut params = CallToolRequestParams::new(tool.to_string());
    if let Some(obj) = args.as_object() {
        params.arguments = Some(obj.clone());
    }
    let result = client
        .call_tool(params)
        .await
        .map_err(|e| format!("call_tool mobile/{tool}: {e:#}"))?;
    let is_error = result.is_error.unwrap_or(false);
    let text = result
        .content
        .first()
        .and_then(|c| c.as_text().map(|t| t.text.clone()))
        .unwrap_or_default();
    let _ = client.cancel().await;
    Ok((names, is_error, text))
}

fn post_mobile_initialize(
    port: u16,
    authorization: Option<&str>,
) -> (u16, String, Option<String>) {
    http_post_json_auth(
        &format!("http://127.0.0.1:{port}/mcp/{MOBILE_PATH}"),
        &initialize_body("t3-mobile"),
        authorization,
        None,
    )
}

/// Normal: REGISTERED_SCENE_SLOTS stays workbench + cursor_ide; mobile is not a registered slot.
#[test]
fn t3_registered_scene_slots_exclude_mobile() {
    assert_eq!(
        super::REGISTERED_SCENE_SLOTS,
        &[WORKBENCH_SLOT, CURSOR_IDE_SLOT]
    );
    assert!(!super::REGISTERED_SCENE_SLOTS.contains(&MOBILE_PATH));
    assert!(!super::is_registered_scene_slot(MOBILE_PATH));
    assert!(build_slot_tool_table(MOBILE_PATH).is_none());
    assert!(tools_list_for_slot(MOBILE_PATH).is_empty());
}

/// Boundary: do not add a mobile arm to slot registration / build_slot_tool_table; no Slot::Mobile.
#[test]
fn t3_scene_slot_registration_and_tool_table_have_no_mobile_arm() {
    let src = adapter_source();
    assert!(
        !src.contains("SceneSlotApi"),
        "slot layer must not use per-slot business flags"
    );
    assert!(
        !src.contains("include_notes"),
        "tool tables must not gate on include_notes"
    );
    let table = source_fn_after(src, "build_slot_tool_table")
        .split("/// Allowlisted tool names")
        .next()
        .expect("build_slot_tool_table body");
    assert!(
        !table.contains("\"mobile\""),
        "build_slot_tool_table must not grow a mobile arm"
    );
    assert!(
        !src.contains("Slot::Mobile"),
        "adapter must not add Slot::Mobile"
    );
    assert_eq!(Slot::parse(MOBILE_PATH), Err(OAuthError::slot_unknown));
}

/// Boundary: /mcp/mobile is nested before catch-all reject_unknown_slot.
#[test]
fn t3_mcp_mobile_nests_before_unknown_slot_catchall() {
    let src = adapter_source();
    assert!(
        src.contains("/mcp/mobile"),
        "adapter must independently nest /mcp/mobile"
    );
    let build = source_fn_after(src, "build_router")
        .split("fn bearer_secret")
        .next()
        .expect("build_router body");
    let mobile_at = build
        .find("/mcp/mobile")
        .expect("/mcp/mobile must appear in build_router before catch-all");
    let catch_at = build
        .find("/mcp/{scene_slot}")
        .expect("catch-all /mcp/{{scene_slot}} must remain");
    assert!(
        mobile_at < catch_at,
        "/mcp/mobile must nest before catch-all reject_unknown_slot"
    );
    assert!(
        !super::REGISTERED_SCENE_SLOTS.contains(&MOBILE_PATH),
        "mobile must not be written into REGISTERED_SCENE_SLOTS"
    );
}

/// Normal: /mcp/mobile door only calls verify_device_token (not verify_for_slot).
#[test]
fn t3_mcp_mobile_door_only_calls_verify_device_token() {
    let src = adapter_source();
    assert!(
        src.contains("verify_device_token"),
        "/mcp/mobile door must call verify_device_token"
    );
    let registered = source_fn_after(src, "verify_registered_slot")
        .split("async fn slot_bearer_gate")
        .next()
        .expect("verify_registered_slot body");
    assert!(
        registered.contains("verify_for_slot"),
        "registered slots still use verify_for_slot"
    );
    assert!(
        !registered.contains("verify_device_token"),
        "registered-slot door must not switch to verify_device_token"
    );
    for chunk in src.split("fn ") {
        if chunk.contains("verify_device_token(") {
            assert!(
                !chunk.contains("verify_for_slot("),
                "the function that calls verify_device_token must not also call verify_for_slot"
            );
        }
    }
}

/// Normal: MCP still listens on 127.0.0.1 only; T3 does not change /health.
#[test]
fn t3_mcp_stays_loopback_and_health_unchanged() {
    let src = adapter_source();
    assert!(
        !src.contains("0.0.0.0"),
        "MCP must keep listening on 127.0.0.1, not 0.0.0.0"
    );
    let health = source_fn_after(src, "health_json")
        .split("async fn bare_mcp_reject")
        .next()
        .expect("health_json body");
    assert!(
        !health.contains("verify_device_token") && !health.contains("verify_for_slot"),
        "T3 must not change /health to consult tickets"
    );
    assert!(!health.contains("mobile"), "T3 must not change /health");

    let handle = start_ephemeral_mcp();
    assert_eq!(handle.local_addr().ip().to_string(), "127.0.0.1");
    let (status, body) = http_get(&format!(
        "http://127.0.0.1:{}/health",
        handle.local_addr().port()
    ));
    assert_eq!(status, 200, "T3 must not change /health, body={body}");
    let json: serde_json::Value = serde_json::from_str(&body).expect("health JSON");
    assert_eq!(json.get("ok"), Some(&serde_json::Value::Bool(true)));
    let mcp = json.get("mcp").and_then(|v| v.as_str()).unwrap_or("");
    assert!(!mcp.trim().is_empty(), "health mcp field must stay non-empty");
    stop_embedded_mcp_runtime(handle).expect("stop");
}

/// Normal: /mcp/mobile is mounted; a live device ticket is admitted (not catch-all 404).
#[test]
fn t3_mcp_mobile_is_mounted_and_accepts_device_ticket() {
    with_device_sandbox(|| {
        let token = issue_device_ticket("phone-t3-mount");
        let handle = start_ephemeral_mcp();
        assert_eq!(handle.local_addr().ip().to_string(), "127.0.0.1");
        let auth = bearer(&token);
        let (status, body, session) =
            post_mobile_initialize(handle.local_addr().port(), Some(&auth));
        assert_ne!(
            status, 404,
            "/mcp/mobile must be mounted before catch-all, body={body}"
        );
        assert_ne!(
            status, 401,
            "live device ticket must pass the mobile door, body={body}"
        );
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            assert_ne!(
                json.get("error").and_then(|v| v.as_str()),
                Some("unknown_scene_slot"),
                "/mcp/mobile must not fall into reject_unknown_slot, body={body}"
            );
        }
        let _ = session;
        stop_embedded_mcp_runtime(handle).expect("stop");
    });
}

/// Normal: live device ticket lists notes∪todo minus workbench-only tools.
#[test]
fn t3_mcp_mobile_tools_match_workbench_table() {
    with_device_sandbox(|| {
        let token = issue_device_ticket("phone-t3-tools");
        let handle = start_ephemeral_mcp();
        let names = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio")
            .block_on(run_initialize_and_list_tools_authed(
                handle.local_addr().port(),
                MOBILE_PATH,
                &token,
            ))
            .expect("/mcp/mobile tools/list");
        let got: BTreeSet<_> = names.iter().map(String::as_str).collect();
        let expected = cursor_ide_expected_tool_names();
        assert_eq!(
            got, expected,
            "/mcp/mobile tools = notes∪todo minus workbench-only (delete_note)"
        );
        assert!(
            !got.contains("get_notes_selection"),
            "/mcp/mobile must not include get_notes_selection"
        );
        assert!(
            !got.contains("delete_note"),
            "/mcp/mobile must not include delete_note"
        );
        let table = build_channel_tool_table(WORKBENCH_SLOT, MOBILE_PATH).expect("mobile channel");
        let table_names: BTreeSet<_> = table.tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(got, table_names);
        assert!(
            build_slot_tool_table(MOBILE_PATH).is_none(),
            "reuse workbench table; do not register a mobile table"
        );
        stop_embedded_mcp_runtime(handle).expect("stop");
    });
}

/// Normal: live device ticket can call the full workbench tool set through /mcp/mobile.
#[test]
fn t3_live_device_ticket_can_call_full_workbench_tools_on_mobile() {
    with_device_sandbox(|| {
        plant_todo_migration_gate();
        plant_empty_notes_index();
        let token = issue_device_ticket("phone-t3-call");
        let handle = start_ephemeral_mcp();
        let port = handle.local_addr().port();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio");

        let cases = [
            ("list_todo_tasks", serde_json::json!({})),
            ("list_todo_categories", serde_json::json!({})),
            ("get_all_notes_catalog", serde_json::json!({})),
        ];
        for (tool, args) in cases {
            let (names, is_error, text) = rt
                .block_on(list_and_call_mobile(port, &token, tool, args))
                .unwrap_or_else(|e| panic!("mobile must be able to call {tool}: {e}"));
            let got: BTreeSet<_> = names.iter().map(String::as_str).collect();
            assert_eq!(
                got,
                cursor_ide_expected_tool_names(),
                "mobile channel set = notes∪todo minus workbench-only"
            );
            assert!(!is_error, "mobile {tool} must succeed, got {text}");
            assert!(
                serde_json::from_str::<serde_json::Value>(&text).is_ok(),
                "mobile {tool} must return Services JSON, got {text}"
            );
        }

        stop_embedded_mcp_runtime(handle).expect("stop");
    });
}

/// Exception: workbench / cursor_ide slot tickets are rejected on /mcp/mobile.
#[test]
fn t3_slot_tickets_are_rejected_on_mcp_mobile() {
    with_device_sandbox(|| {
        let handle = start_ephemeral_mcp();
        let port = handle.local_addr().port();
        for slot in [Slot::Workbench, Slot::CursorIde] {
            let ticket = issue_for_slot(slot).expect("issue slot ticket");
            let auth = bearer(&ticket);
            let (status, body, session) = post_mobile_initialize(port, Some(&auth));
            assert_uniform_401(status, &body, Some(ticket.as_str()));
            assert!(
                session.is_none(),
                "slot ticket must not mint a /mcp/mobile session"
            );
        }
        stop_embedded_mcp_runtime(handle).expect("stop");
    });
}

/// Exception: revoked and missing device tickets are rejected; unbound cannot call tools.
#[test]
fn t3_revoked_missing_and_unbound_cannot_call_mobile_tools() {
    with_device_sandbox(|| {
        let handle = start_ephemeral_mcp();
        let port = handle.local_addr().port();

        let (status, body, session) = post_mobile_initialize(port, None);
        assert_uniform_401(status, &body, None);
        assert!(session.is_none(), "unbound / no ticket must not mint a session");

        let missing = TicketHandle::from_secret("00".repeat(32));
        let missing_auth = bearer(&missing);
        let (status, body, session) = post_mobile_initialize(port, Some(&missing_auth));
        assert_uniform_401(status, &body, Some(missing.as_str()));
        assert!(session.is_none(), "unknown device ticket must not mint a session");

        let live = issue_device_ticket("phone-t3-revoke");
        revoke_for_device("phone-t3-revoke").expect("revoke");
        let revoked_auth = bearer(&live);
        let (status, body, session) = post_mobile_initialize(port, Some(&revoked_auth));
        assert_uniform_401(status, &body, Some(live.as_str()));
        assert!(session.is_none(), "revoked device ticket must not mint a session");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio");
        let err = rt.block_on(run_initialize_and_list_tools_authed(
            port,
            MOBILE_PATH,
            &missing,
        ));
        assert!(
            err.is_err(),
            "unbound / missing ticket must not list or call any mobile tool"
        );

        stop_embedded_mcp_runtime(handle).expect("stop");
    });
}

/// Normal: unchecked tools are omitted from /mcp/mobile tools/list and rejected on call.
#[test]
fn channel_filter_hides_and_rejects_create_note_on_mobile() {
    with_device_sandbox(|| {
        let all = workbench_expected_tool_names();
        let enabled: Vec<String> = all
            .iter()
            .copied()
            .filter(|name| *name != "create_note")
            .map(str::to_string)
            .collect();
        crate::services::settings::mcp_channel_tools::set_enabled("mobile", enabled).expect("save");

        let token = issue_device_ticket("phone-channel-filter");
        let handle = start_ephemeral_mcp();
        let port = handle.local_addr().port();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("tokio");

        let names = rt
            .block_on(run_initialize_and_list_tools_authed(
                port,
                MOBILE_PATH,
                &token,
            ))
            .expect("mobile list");
        let got: BTreeSet<_> = names.iter().map(String::as_str).collect();
        assert!(!got.contains("create_note"), "unchecked tool must leave list");
        assert!(got.contains("get_all_notes_catalog"));
        assert_ne!(got, all, "mobile list must no longer equal the full table");

        let err = rt.block_on(list_and_call_mobile(
            port,
            &token,
            "create_note",
            serde_json::json!({"source_path": "/tmp/x.md"}),
        ));
        assert!(
            err.is_err(),
            "call_tool must reject an unchecked mobile tool, got {err:?}"
        );

        stop_embedded_mcp_runtime(handle).expect("stop");
    });
}

/// Plan B: same MCP name, mobile channel invokes create_note_content.
#[test]
fn mobile_channel_create_note_hits_content_api() {
    let table = build_channel_tool_table(WORKBENCH_SLOT, MOBILE_PATH).expect("overlay");
    let create = table
        .tools
        .iter()
        .find(|t| t.name == "create_note")
        .expect("create_note");
    assert!(
        create.invoke == create_note_from_content,
        "mobile create_note must invoke create_note_content"
    );
    let required = create.input_schema["required"]
        .as_array()
        .expect("required");
    assert!(required.iter().any(|v| v.as_str() == Some("content")));
    assert!(required.iter().any(|v| v.as_str() == Some("title")));
    assert!(required.iter().any(|v| v.as_str() == Some("digest")));
    assert!(!required.iter().any(|v| v.as_str() == Some("source_path")));

    let path_table = build_channel_tool_table(WORKBENCH_SLOT, WORKBENCH_SLOT).expect("path");
    let path_create = path_table
        .tools
        .iter()
        .find(|t| t.name == "create_note")
        .expect("create_note");
    assert!(
        path_create.invoke == create_note_from_source,
        "workbench create_note must invoke create_note from source_path"
    );
    let path_required = path_create.input_schema["required"]
        .as_array()
        .expect("required");
    assert!(path_required.iter().any(|v| v.as_str() == Some("source_path")));
    assert!(path_required.iter().any(|v| v.as_str() == Some("title")));
    assert!(path_required.iter().any(|v| v.as_str() == Some("digest")));
}

/// Exception: oauth still rejects mobile; T3 failure must not change Slot.
#[test]
fn t3_oauth_still_rejects_mobile_and_does_not_change_slot() {
    assert_eq!(Slot::parse(MOBILE_PATH), Err(OAuthError::slot_unknown));
    let src = adapter_source();
    assert!(!src.contains("Slot::Mobile"));
    assert!(!super::is_registered_scene_slot(MOBILE_PATH));
    let oauth = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mcp_oauth.rs"
    ));
    assert!(
        !oauth.contains("Slot::Mobile"),
        "T3 must not add Slot::Mobile"
    );
}

fn note_content_tool(table: &SlotToolTable) -> &ToolRoute {
    table
        .tools
        .iter()
        .find(|t| t.name == "get_note_content")
        .expect("get_note_content")
}

/// Normal: workbench channel hangs get_note_content on note_path Services.
#[test]
fn workbench_channel_get_note_content_hits_note_path() {
    let table = build_channel_tool_table(WORKBENCH_SLOT, WORKBENCH_SLOT).expect("workbench");
    let tool = note_content_tool(&table);
    assert_eq!(tool.name, "get_note_content");
    assert!(
        tool.invoke == note_path_invoke,
        "workbench get_note_content must invoke note_path"
    );
    assert!(
        tool.description.contains("staged document id") && tool.description.contains("F1"),
        "workbench description must say the return is a Stage document id, got {}",
        tool.description
    );
    assert!(
        !tool.description.to_ascii_lowercase().contains("absolute path"),
        "workbench description must not tell the model to expect a path: {}",
        tool.description
    );
}

/// Normal: cursor_ide and mobile hang get_note_content on note_path (absolute path, no body).
/// Boundary: /mcp/mobile still uses workbench scene_slot; description split is by channel, not slot.
#[test]
fn cursor_ide_and_mobile_get_note_content_hit_note_path() {
    assert!(!super::is_registered_scene_slot(MOBILE_PATH));
    assert_eq!(
        super::REGISTERED_SCENE_SLOTS,
        &[WORKBENCH_SLOT, CURSOR_IDE_SLOT]
    );

    let ide = build_channel_tool_table(CURSOR_IDE_SLOT, CURSOR_IDE_SLOT).expect("cursor_ide");
    let ide_tool = note_content_tool(&ide);
    assert_eq!(ide_tool.name, "get_note_content");
    assert!(
        ide_tool.invoke == note_path_invoke,
        "cursor_ide get_note_content must invoke note_path"
    );
    assert!(
        ide_tool.description.contains("absolute file path")
            && ide_tool.description.contains("Does not return file body"),
        "cursor_ide description must say absolute path and no body, got {}",
        ide_tool.description
    );
    assert!(
        !ide_tool.description.contains("Stage"),
        "cursor_ide description must not mention Stage: {}",
        ide_tool.description
    );

    let mobile = build_channel_tool_table(WORKBENCH_SLOT, MOBILE_PATH).expect("mobile overlay");
    let mobile_tool = note_content_tool(&mobile);
    assert_eq!(mobile_tool.name, "get_note_content");
    assert!(
        mobile_tool.invoke == note_path_invoke,
        "mobile get_note_content must invoke note_path"
    );
    assert!(
        mobile_tool.description.contains("absolute file path")
            && !mobile_tool.description.contains("staged document id"),
        "mobile must not inherit workbench Stage description from scene_slot=workbench"
    );
}

/// Product hard-gate: delete_note only on workbench channel.
#[test]
fn delete_note_is_workbench_channel_only() {
    let wb = build_channel_tool_table(WORKBENCH_SLOT, WORKBENCH_SLOT).expect("workbench");
    assert!(
        wb.tools.iter().any(|t| t.name == "delete_note"),
        "workbench must expose delete_note"
    );

    let ide = build_channel_tool_table(CURSOR_IDE_SLOT, CURSOR_IDE_SLOT).expect("cursor_ide");
    assert!(
        ide.tools.iter().all(|t| t.name != "delete_note"),
        "cursor_ide must not list delete_note"
    );

    let mobile = build_channel_tool_table(WORKBENCH_SLOT, MOBILE_PATH).expect("mobile");
    assert!(
        mobile.tools.iter().all(|t| t.name != "delete_note"),
        "mobile must not list delete_note even with scene_slot=workbench"
    );

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    let err = rt.block_on(proxy_tool_call(
        CURSOR_IDE_SLOT,
        CURSOR_IDE_SLOT,
        "delete_note",
        serde_json::json!({ "id": "11111111111111111111111111111111" }),
    ));
    assert!(
        err.is_err(),
        "cursor_ide delete_note must fail allowlist before Services"
    );
}

/// Directory extract: MCP Server is an L1 crate-root module, not under Services.
#[test]
fn crate_registers_mcp_host_as_in_process_module() {
    let lib = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    let services = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mod.rs"
    ));
    assert!(
        lib.contains("pub mod mcp_host"),
        "lib.rs must register MCP Server at crate root"
    );
    assert!(
        !services.contains("pub mod mcp_host"),
        "MCP Server must not remain under services/"
    );
}

/// Boundary: MCP name is get_note_content; digest still hangs get_note_digest_by_id.
#[test]
fn get_note_content_name_and_digest_route() {
    for (slot, channel) in [
        (WORKBENCH_SLOT, WORKBENCH_SLOT),
        (CURSOR_IDE_SLOT, CURSOR_IDE_SLOT),
        (WORKBENCH_SLOT, MOBILE_PATH),
    ] {
        let table = build_channel_tool_table(slot, channel).expect("table");
        assert!(
            table.tools.iter().any(|t| t.name == "get_note_content"),
            "{channel} must expose MCP name get_note_content"
        );
        assert!(
            table.tools.iter().all(|t| t.name != "get_note_content_by_id"),
            "{channel} must not keep retired name get_note_content_by_id"
        );
        let digest = table
            .tools
            .iter()
            .find(|t| t.name == "get_note_digest_by_id")
            .expect("get_note_digest_by_id");
        assert_eq!(digest.name, "get_note_digest_by_id");
        assert!(
            digest.invoke as usize != 0,
            "{channel} digest must keep a Services invoke"
        );
    }
}

#[path = "mcp_host/ac1.rs"]
mod ac1;

