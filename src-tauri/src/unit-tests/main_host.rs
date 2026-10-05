use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use reqwest::blocking;
use serde_json::{json, Value};

use super::*;
use crate::services::bind::{
    complete_bind, create_bind_payload, seal_bind_request, test_clear_session,
    test_reset_bind_keychain,
};
use crate::test_support::TestSandbox;

struct RepoFixture {
    _sandbox: TestSandbox,
    repo_root: PathBuf,
}

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

fn setup_repo_without_index() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.spark_root();
    fs::create_dir_all(&wb).expect("mkdir spark");
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

fn setup_repo_with_notes() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let notes = sandbox.data_dir().join("notes");
    fs::create_dir_all(notes.join("digest")).expect("mkdir digest");
    fs::write(notes.join("index.json"), br#"{"entries":[]}"#).expect("index");
    fs::write(notes.join("digest/note.md"), b"digest body").expect("digest file");
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

fn setup_repo_for_read_later() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.spark_root();
    fs::create_dir_all(&wb).expect("mkdir spark");
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

const MIGRATION_GATE_FILE: &str = ".migration_gate_passed";

fn plant_migration_gate(wb: &std::path::Path) {
    let todo_root = wb.join("todo_tasks");
    fs::create_dir_all(&todo_root).expect("mkdir todo_tasks for gate");
    fs::write(todo_root.join(MIGRATION_GATE_FILE), b"ok\n").expect("write migration gate");
}

fn setup_repo_for_todo_task() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.spark_root();
    fs::create_dir_all(&wb).expect("mkdir spark");
    // Happy-path todo HTTP tests assume migration already succeeded (t5 wrote the marker).
    plant_migration_gate(&wb);
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

fn http_get(port: u16, path: &str) -> (u16, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let response = blocking::get(&url).expect("http get");
    let status = response.status().as_u16();
    let body: Value = response.json().unwrap_or(json!({}));
    (status, body)
}

fn http_post(port: u16, path: &str, payload: &Value) -> (u16, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = blocking::Client::new();
    let response = client
        .post(&url)
        .json(payload)
        .send()
        .expect("http post");
    let status = response.status().as_u16();
    let body: Value = response.json().unwrap_or(json!({}));
    (status, body)
}

fn http_put(port: u16, path: &str, payload: &Value) -> (u16, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = blocking::Client::new();
    let response = client
        .put(&url)
        .json(payload)
        .send()
        .expect("http put");
    let status = response.status().as_u16();
    let body: Value = response.json().unwrap_or(json!({}));
    (status, body)
}

fn http_post_with_response(
    port: u16,
    path: &str,
    payload: &Value,
) -> (u16, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = blocking::Client::new();
    let response = client
        .post(&url)
        .json(payload)
        .send()
        .expect("http post");
    let status = response.status().as_u16();
    assert_cors_headers(&response);
    let body: Value = response.json().unwrap_or(json!({}));
    (status, body)
}

fn http_patch(port: u16, path: &str, payload: &Value) -> (u16, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = blocking::Client::new();
    let response = client
        .patch(&url)
        .json(payload)
        .send()
        .expect("http patch");
    let status = response.status().as_u16();
    let body: Value = response.json().unwrap_or(json!({}));
    (status, body)
}

fn http_options(port: u16, path: &str) -> (u16, blocking::Response) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = blocking::Client::new();
    let response = client
        .request(reqwest::Method::OPTIONS, &url)
        .send()
        .expect("http options");
    let status = response.status().as_u16();
    (status, response)
}

fn assert_cors_headers(response: &blocking::Response) {
    assert_eq!(
        response
            .headers()
            .get("Access-Control-Allow-Origin")
            .and_then(|v| v.to_str().ok()),
        Some("*")
    );
    assert_eq!(
        response
            .headers()
            .get("Access-Control-Allow-Methods")
            .and_then(|v| v.to_str().ok()),
        Some("POST, OPTIONS")
    );
    assert_eq!(
        response
            .headers()
            .get("Access-Control-Allow-Headers")
            .and_then(|v| v.to_str().ok()),
        Some("Content-Type")
    );
}

fn with_server<F: FnOnce(u16)>(repo_root: PathBuf, f: F) {
    let port = ephemeral_port();
    let handle = start(repo_root, port).expect("start server");
    thread::sleep(Duration::from_millis(50));
    f(port);
    stop(handle);
}

#[test]
fn post_bind_complete_issues_ticket_via_sidecar() {
    let fixture = setup_repo_without_index();
    test_reset_bind_keychain();
    test_clear_session();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let payload = create_bind_payload(
            "10.0.0.4".parse().expect("ip"),
            17654,
            &"ab".repeat(32),
        )
        .expect("draw");
        let sealed =
            seal_bind_request(&payload.temp_pub, "phone-http", Some("Pixel")).expect("seal");
        let url = format!("http://127.0.0.1:{port}/api/bind-complete");
        let response = blocking::Client::new()
            .post(&url)
            .header("content-type", "application/octet-stream")
            .body(sealed.clone())
            .send()
            .expect("bind complete");
        assert_eq!(response.status().as_u16(), 200);
        let body: Value = response.json().expect("json");
        assert_eq!(body["device_mcp_token"].as_str().map(str::len), Some(64));
        assert_eq!(
            complete_bind(&sealed).expect_err("consumed"),
            crate::services::bind::BindError::consumed
        );
    });
    test_clear_session();
}

#[test]
fn start_fails_when_port_in_use_without_panic() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    let port = ephemeral_port();
    let _guard = TcpListener::bind(format!("127.0.0.1:{port}")).expect("occupy port");
    let result = start(repo_root, port);
    assert!(result.is_err());
}


#[test]
fn post_unknown_returns_404() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/unknown");
        assert_eq!(status, 404);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn map_value_to_response_strips_status_and_uses_code() {
    let (code, body) = map_value_to_response(json!({
        "error": "missing",
        "_status": 404
    }));
    assert_eq!(code, 404);
    let parsed: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(parsed["error"], "missing");
    assert!(parsed.get("_status").is_none());
}

#[test]
fn default_http_port_is_8765() {
    assert_eq!(DEFAULT_HTTP_PORT, 8765);
}

#[test]
fn main_host_state_start_sets_http_ready_and_listens() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    let state = MainHostState::new();
    let port = ephemeral_port();
    state.try_start(repo_root, port);
    assert!(state.is_ready());
    thread::sleep(Duration::from_millis(50));
    let (status, _) = http_get(port, "/api/unknown");
    assert_eq!(status, 404);
    state.stop();
}

#[test]
fn main_host_state_stop_clears_http_ready_and_releases_port() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    let state = MainHostState::new();
    let port = ephemeral_port();
    state.try_start(repo_root, port);
    assert!(state.is_ready());
    thread::sleep(Duration::from_millis(50));
    state.stop();
    assert!(!state.is_ready());
    assert!(!crate::wait_for_port(port, Duration::from_millis(200)));
}

#[test]
fn main_host_state_three_cycles_no_port_leak() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    let state = MainHostState::new();
    let port = ephemeral_port();
    for _ in 0..3 {
        state.try_start(repo_root.clone(), port);
        assert!(state.is_ready());
        thread::sleep(Duration::from_millis(50));
        assert!(crate::wait_for_port(port, Duration::from_millis(500)));
        state.stop();
        assert!(!state.is_ready());
        assert!(!crate::wait_for_port(port, Duration::from_millis(200)));
    }
}

#[test]
fn main_host_state_bind_failure_keeps_http_ready_false() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    let port = ephemeral_port();
    let _guard = TcpListener::bind(format!("127.0.0.1:{port}")).expect("occupy port");
    let state = MainHostState::new();
    state.try_start(repo_root, port);
    assert!(!state.is_ready());
}

#[test]
fn post_read_later_creates_entry_with_cors() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post_with_response(
            port,
            "/api/read-later",
            &json!({ "url": "https://example.com/a", "title": "Example" }),
        );
        assert_eq!(status, 201);
        let entry = &body["entry"];
        assert_eq!(entry["url"], "https://example.com/a");
        assert_eq!(entry["title"], "Example");
        assert_eq!(entry["read"], false);
        assert!(entry.get("id").and_then(|v| v.as_str()).is_some());
        assert!(entry.get("saved_at").and_then(|v| v.as_str()).is_some());
    });
}

#[test]
fn post_read_later_returns_409_for_recent_duplicate() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let payload = json!({ "url": "https://example.com/duplicate", "title": "Example" });
        let (first_status, _) = http_post_with_response(port, "/api/read-later", &payload);
        assert_eq!(first_status, 201);

        let (status, body) = http_post_with_response(port, "/api/read-later", &payload);
        assert_eq!(status, 409);
        assert_eq!(body["code"], "read_later_recent_duplicate");
        assert!(body["entry"].is_object());
    });
}

#[test]
fn options_read_later_returns_204_with_cors() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, response) = http_options(port, "/api/read-later");
        assert_eq!(status, 204);
        assert_cors_headers(&response);
    });
}

#[test]
fn post_read_later_missing_url_returns_400_with_cors() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) =
            http_post_with_response(port, "/api/read-later", &json!({ "title": "no url" }));
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn patch_non_read_later_path_returns_405() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, _) =
            http_patch(port, "/api/status", &json!({ "read": true }));
        assert_eq!(status, 405);
    });
}

#[test]
fn options_non_read_later_path_returns_405() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, _) = http_options(port, "/api/status");
        assert_eq!(status, 405);
    });
}

const HTTP_CREATE_TODO_MD: &str = "HTTP fixture body";

fn create_todo_master(title: &str, sub_titles: &[&str]) -> (String, String, Value) {
    // Binding / path fixture IDs only — todo_task service is removed (t8).
    let master_id = format!("bind-{title}");
    let sub_id = sub_titles
        .first()
        .map(|s| format!("sub-{s}"))
        .unwrap_or_default();
    let created = json!({
        "master_task_id": master_id,
        "sub_task_id": sub_id,
        "title": title,
        "body": HTTP_CREATE_TODO_MD,
    });
    (master_id, sub_id, created)
}

#[test]
fn old_plan_http_paths_unavailable_and_complete_sub_unmapped() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, sub_id, _) = create_todo_master("Hard cut", &["Sub A"]);

        // Old GET collection/single paths must not be registered (404).
        for path in ["/api/plan-tasks", &format!("/api/plan-task?id={master_id}")] {
            let (status, body) = http_get(port, path);
            assert_eq!(status, 404, "old GET path must be unavailable: {path}");
            assert!(body.get("error").is_some());
            assert!(body.get("_status").is_none());
        }

        // Old POST action family must not be registered or silently forwarded.
        for path in [
            "/api/plan-task-create",
            "/api/plan-task-delete",
            "/api/plan-task-add-sub",
            "/api/plan-task-delete-sub",
            "/api/plan-task-complete",
            "/api/plan-task-set-status",
            "/api/plan-task-link-archive",
            "/api/plan-task-add-attachment",
            "/api/plan-task-list-attachments",
            "/api/plan-task-get-attachment",
            "/api/plan-task-update-attachment",
            "/api/plan-task-complete-sub",
        ] {
            let payload = match path {
                "/api/plan-task-create" => json!({ "title": "Should fail" }),
                "/api/plan-task-delete" | "/api/plan-task-list-attachments" => {
                    json!({ "master_task_id": master_id })
                }
                "/api/plan-task-add-sub" => {
                    json!({ "master_task_id": master_id, "title": "Sub" })
                }
                "/api/plan-task-set-status" => {
                    json!({ "master_task_id": master_id, "status": "complete" })
                }
                "/api/plan-task-add-attachment"
                | "/api/plan-task-get-attachment"
                | "/api/plan-task-update-attachment" => {
                    json!({
                        "master_task_id": master_id,
                        "file_name": "notes.md",
                        "content": "x",
                    })
                }
                _ => json!({
                    "master_task_id": master_id,
                    "sub_task_id": sub_id,
                    "archive_id": "11111111111111111111111111111111",
                }),
            };
            let (status, body) = http_post(port, path, &payload);
            assert!(
                status == 404 || status == 405,
                "old POST path must be unavailable (404/405), got {status} for {path}: {body}"
            );
            assert!(body.get("error").is_some());
            // Must not silently succeed as a todo handler.
            assert_ne!(status, 200);
            assert_ne!(status, 201);
        }

        // complete-sub is deleted and must not be remapped under todo-*.
        let (status, body) = http_post(
            port,
            "/api/todo-task-complete-sub",
            &json!({ "master_task_id": master_id, "sub_task_id": sub_id }),
        );
        assert!(
            status == 404 || status == 405,
            "todo-task-complete-sub must stay unmapped, got {status}: {body}"
        );
        assert!(body.get("error").is_some());
    });
}

// --- notes-selection stack removed ---

const NOTES_SELECTION_API: &str = "/api/notes-selection";

fn t5_repo_file(rel: &str) -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Exception: Sidecar no longer serves GET/PUT /api/notes-selection.
#[test]
fn notes_selection_http_route_is_gone() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(get_status, 404, "GET /api/notes-selection must be gone: {get_body}");
        let (put_status, put_body) = http_put(
            port,
            NOTES_SELECTION_API,
            &json!({ "date": null, "documents": [] }),
        );
        assert!(
            put_status == 404 || put_status == 405,
            "PUT /api/notes-selection must be gone, got {put_status}: {put_body}"
        );
        assert_ne!(put_status, 200);
    });
}

/// Directory extract: Main Host is an L1 crate-root module, not under Services.
#[test]
fn crate_registers_main_host_as_in_process_module() {
    let lib = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    let services = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mod.rs"
    ));
    assert!(
        lib.contains("pub mod main_host"),
        "lib.rs must register Main Host at crate root"
    );
    assert!(
        !services.contains("pub mod main_host"),
        "Main Host must not remain under services/"
    );
}

/// Exception: Host snapshot module and MCP notes-selection tools are gone.
#[test]
fn notes_selection_module_and_mcp_tool_are_gone() {
    let services = t5_repo_file("src-tauri/src/services/mod.rs");
    assert!(
        !services.contains("notes_selection"),
        "services/mod.rs must not mount notes_selection"
    );
    let adapter = crate::test_support::read_rs_dir({
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop();
        p.push("src-tauri/src/mcp_host");
        p
    });
    assert!(
        !adapter.contains("get_notes_selection") && !adapter.contains("NOTES_SLOT_ONLY_TOOLS"),
        "MCP adapter must not hang get_notes_selection"
    );
    let http = crate::test_support::read_rs_dir({
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop();
        p.push("src-tauri/src/main_host");
        p
    });
    assert!(
        !http.contains("/api/notes-selection") && !http.contains("notes_selection"),
        "Sidecar must not serve /api/notes-selection"
    );
    let notes_mod = {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.push("src-tauri/src/services/notes_selection.rs");
        path
    };
    assert!(
        !notes_mod.exists(),
        "notes_selection.rs must be deleted"
    );
    for slot in ["spark", "cursor_ide", "todo_task", "notes"] {
        if let Some(table) = crate::mcp_host::build_slot_tool_table(slot) {
            for tool in &table.tools {
                let name = tool.name.to_lowercase();
                assert!(
                    !name.contains("notes_selection") && !name.contains("notes-selection"),
                    "no notes-selection MCP tool, found {} on {slot}",
                    tool.name
                );
            }
        }
    }
}

/// Exception: frontend no longer writes a notes-selection snapshot.
#[test]
fn frontend_does_not_write_notes_selection_snapshot() {
    for rel in [
        "frontend/src/host/apiClient.ts",
        "frontend/src/boot.ts",
        "frontend/src/notes/ui/sidebar.tsx",
        "frontend/src/notes/commands/sidebar.ts",
        "frontend/src/notes/ui/cards.tsx",
        "frontend/src/notes/commands/cards.ts",
        "frontend/src/notes/commands/assistant.ts",
        "frontend/src/notes/commands/delete-dialog.ts",
        "frontend/src/notes/commands/settle-dialog.ts",
        "frontend/src/notes/commands/move-project-dialog.ts",
        "frontend/src/notes/commands/viewer/doc.ts",
        "frontend/src/notes/commands/viewer/create.ts",
        "frontend/src/host/writeApiInvokeMap.ts",
        "frontend/src/main.tsx",
        "frontend/src/hash-router.tsx",
        "frontend/src/toast.tsx",
    ] {
        let src = t5_repo_file(rel);
        assert!(
            !src.contains("writeNotesSelectionSnapshot")
                && !src.contains("clearNotesSelectionSnapshot")
                && !src.contains("/api/notes-selection")
                && !src.contains("notes_selection"),
            "{rel} must not write notes-selection"
        );
    }
}

/// Exception: do not add a frontend notes-selection test harness.
#[test]
fn t5_does_not_add_frontend_notes_selection_test_harness() {
    let pkg = t5_repo_file("package.json");
    assert!(
        !pkg.contains("notes-selection")
            && !pkg.contains("writeNotesSelection")
            && !pkg.contains("notes-selection-snapshot"),
        "must not add a new frontend notes-selection test file to npm test"
    );
    let tests_dir = {
        let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop();
        path.push("tests");
        path
    };
    let extra = std::fs::read_dir(&tests_dir)
        .expect("tests dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            let lower = name.to_lowercase();
            lower.contains("notes-selection") || lower.contains("notes_selection")
        })
        .collect::<Vec<_>>();
    assert!(
        extra.is_empty(),
        "must not add a new frontend test harness, found {extra:?}"
    );
}

#[path = "main_host/produce_wiring.rs"]
mod produce_wiring;

#[path = "main_host/ac1.rs"]
mod ac1;
