use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use reqwest::blocking;
use serde_json::{json, Value};

use super::*;
use crate::test_support::TestSandbox;

fn assert_ac5_master_task_shape(task: &Value) {
    assert!(task.get("master_task_id").and_then(|v| v.as_str()).is_some());
    assert!(task.get("title").and_then(|v| v.as_str()).is_some());
    let status = task.get("status").and_then(|v| v.as_str()).expect("status");
    assert!(
        TODO_TASK_MASTER_STATUS_WIRE.contains(&status),
        "locked read exit status must be tri-state wire value, got {status}"
    );
    assert!(task.get("created_at").and_then(|v| v.as_str()).is_some());
    let subs = task["sub_tasks"].as_array().expect("sub_tasks");
    assert!(!subs.is_empty());
    for sub in subs {
        assert!(sub.get("sub_task_id").and_then(|v| v.as_str()).is_some());
        assert!(sub.get("status").and_then(|v| v.as_str()).is_some());
        assert!(sub.get("implicit").map(|v| v.is_boolean()).unwrap_or(false));
        let links = sub["linked_archive_ids"].as_array().expect("linked_archive_ids");
        assert!(links.is_empty() || !links.is_empty());
        assert!(
            sub.get("completed_at").is_none() || sub["completed_at"].is_null(),
            "create/list must omit or null completed_at"
        );
    }
}

struct RepoFixture {
    _sandbox: TestSandbox,
    repo_root: PathBuf,
}

struct CatalogFixture {
    _sandbox: TestSandbox,
    repo_root: PathBuf,
    id: String,
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
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(&wb).expect("mkdir corpus");
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

fn setup_repo_with_corpus() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(wb.join("digest")).expect("mkdir digest");
    fs::write(wb.join("index.json"), br#"{"entries":[]}"#).expect("index");
    fs::write(wb.join("digest/note.md"), b"digest body").expect("digest file");
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

fn setup_repo_for_read_later() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(&wb).expect("mkdir corpus");
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
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(&wb).expect("mkdir corpus");
    // Happy-path todo HTTP tests assume migration already succeeded (t5 wrote the marker).
    plant_migration_gate(&wb);
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

fn setup_repo_for_todo_task_without_gate() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(&wb).expect("mkdir corpus");
    RepoFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        _sandbox: sandbox,
    }
}

fn setup_repo_with_catalog() -> CatalogFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(wb.join("digest/ai")).expect("mkdir digest");
    let id = "11111111111111111111111111111111";
    let index = json!({
        "entries": {
            id: {
                "common_path": "ai/note.md",
                "created_at": "202606190004",
                "layers": ["digest"]
            }
        }
    });
    fs::write(wb.join("index.json"), index.to_string()).expect("index");
    fs::write(wb.join("digest/ai/note.md"), b"digest body").expect("digest file");
    CatalogFixture {
        repo_root: sandbox.config_dir().to_path_buf(),
        id: id.to_string(),
        _sandbox: sandbox,
    }
}

fn setup_repo_for_archive() -> RepoFixture {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(wb.join("raw")).expect("mkdir raw");
    fs::create_dir_all(wb.join("digest")).expect("mkdir digest");
    fs::write(wb.join("index.json"), br#"{"entries":{}}"#).expect("index");
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

fn http_get_with_response(port: u16, path: &str) -> (u16, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let response = blocking::get(&url).expect("http get");
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

fn http_patch_with_response(
    port: u16,
    path: &str,
    payload: &Value,
) -> (u16, Value) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let client = blocking::Client::new();
    let response = client
        .patch(&url)
        .json(payload)
        .send()
        .expect("http patch");
    let status = response.status().as_u16();
    assert_cors_headers(&response);
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
        Some("GET, POST, PATCH, OPTIONS")
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
fn get_corpus_catalog_latest_per_topic() {
    let catalog = setup_repo_with_catalog();
    let repo_root = catalog.repo_root.clone();
    let id = catalog.id.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-catalog?mode=latest_per_topic");
        assert_eq!(status, 200);
        let items = body["items"].as_array().expect("items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], id);
        assert_eq!(items[0]["topic"], "ai");
        assert!(items[0].get("common_path").is_none());
    });
}

#[test]
fn get_corpus_catalog_unsupported_mode_returns_400() {
    let catalog = setup_repo_with_catalog();
    let repo_root = catalog.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-catalog?mode=unknown");
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn post_corpus_files_returns_batch() {
    let catalog = setup_repo_with_catalog();
    let repo_root = catalog.repo_root.clone();
    let id = catalog.id.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/corpus-files",
            &json!({ "ids": [id, "22222222222222222222222222222222"] }),
        );
        assert_eq!(status, 200);
        let items = body["items"].as_array().expect("items");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["ok"], true);
        assert_eq!(items[0]["content"], "digest body");
        assert_eq!(items[1]["ok"], false);
    });
}

#[test]
fn post_corpus_files_empty_ids_returns_400() {
    let catalog = setup_repo_with_catalog();
    let repo_root = catalog.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(port, "/api/corpus-files", &json!({ "ids": [] }));
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn get_corpus_index_matches_workbench_read() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    let expected = crate::services::workbench_read::get_corpus_index(&repo_root);
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-index");
        assert_eq!(status, 200);
        assert_eq!(body, expected);
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn get_corpus_file_digest_returns_content() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-file?layer=digest&path=note.md");
        assert_eq!(status, 200);
        assert_eq!(body["content"], "digest body");
    });
}

#[test]
fn get_status_returns_ok_and_http_port() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    let port = ephemeral_port();
    let handle = start(repo_root, port).expect("start");
    thread::sleep(Duration::from_millis(50));
    let (status, body) = http_get(port, "/api/status");
    stop(handle);
    assert_eq!(status, 200);
    assert_eq!(body["ok"], true);
    assert_eq!(body["http_port"], port);
}

#[test]
fn get_corpus_file_raw_layer_returns_400() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-file?layer=raw&path=x");
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn get_corpus_asset_raw_returns_base64_png() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(wb.join("raw/ai")).expect("mkdir raw");
    fs::write(wb.join("raw/ai/note.png"), b"\x89PNG\r\n").expect("png");
    fs::write(wb.join("raw/ai/note.md"), b"# note").expect("md");
    let repo_root = sandbox.config_dir().to_path_buf();
    with_server(repo_root, |port| {
        let q = "/api/corpus-asset?layer=raw&base=ai/note.md&href=note.png";
        let (status, body) = http_get(port, q);
        assert_eq!(status, 200);
        assert!(body.get("data_b64").and_then(|x| x.as_str()).is_some());
        assert_eq!(body["mime_type"], "image/png");
    });
}

#[test]
fn maps_workbench_read_status_404_without_status_in_body() {
    let fixture = setup_repo_without_index();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-index");
        assert_eq!(status, 404);
        assert!(body.get("error").is_some());
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn start_fails_when_port_in_use_without_panic() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    let port = ephemeral_port();
    let _guard = TcpListener::bind(format!("127.0.0.1:{port}")).expect("occupy port");
    let result = start(repo_root, port);
    assert!(result.is_err());
}

const SAMPLE_DOC: &str = r#"# Test Title

> 创建时间：2026年6月19日 14:30
> 来源：theme-summary
> 导航：[digest](../../../digest/inbox/test-topic/202606191430-test-slug.md)

---

Summary body here.
"#;

#[test]
fn post_archive_document_and_digest() {
    let fixture = setup_repo_for_archive();
    let stage_dir = fixture._sandbox.cache_dir().join("archive_source_stage");
    fs::create_dir_all(&stage_dir).expect("stage dir");
    let source = stage_dir.join("source.md");
    fs::write(&source, SAMPLE_DOC).expect("write source");
    let source_path = source.canonicalize().expect("canon");
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/archive-document",
            &json!({
                "source_path": source_path.to_str().unwrap(),
                "source_type": "summary",
            }),
        );
        assert_eq!(status, 200);
        assert_eq!(body["ok"], true);
        let id = body["id"].as_str().expect("id");
        let (d_status, d_body) = http_post(
            port,
            "/api/archive-digest",
            &json!({ "id": id, "digest": "# T — 摘要\n\n## 概述\n\nok" }),
        );
        assert_eq!(d_status, 200);
        assert_eq!(d_body["ok"], true);
    });
}

#[test]
fn post_archive_document_rejects_document_field() {
    let fixture = setup_repo_for_archive();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/archive-document",
            &json!({ "document": SAMPLE_DOC }),
        );
        assert_eq!(status, 400);
        assert!(
            body["error"]
                .as_str()
                .unwrap_or("")
                .contains("source_path"),
            "{body}"
        );
    });
}

#[test]
fn post_unknown_returns_404() {
    let fixture = setup_repo_with_corpus();
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
fn local_http_state_start_sets_http_ready_and_listens() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    let state = LocalHttpState::new();
    let port = ephemeral_port();
    state.try_start(repo_root, port);
    assert!(state.is_ready());
    thread::sleep(Duration::from_millis(50));
    let (status, _) = http_get(port, "/api/corpus-index");
    assert_eq!(status, 200);
    state.stop();
}

#[test]
fn local_http_state_stop_clears_http_ready_and_releases_port() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    let state = LocalHttpState::new();
    let port = ephemeral_port();
    state.try_start(repo_root, port);
    assert!(state.is_ready());
    thread::sleep(Duration::from_millis(50));
    state.stop();
    assert!(!state.is_ready());
    assert!(!crate::wait_for_port(port, Duration::from_millis(200)));
}

#[test]
fn local_http_state_three_cycles_no_port_leak() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    let state = LocalHttpState::new();
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
fn local_http_state_bind_failure_keeps_http_ready_false() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    let port = ephemeral_port();
    let _guard = TcpListener::bind(format!("127.0.0.1:{port}")).expect("occupy port");
    let state = LocalHttpState::new();
    state.try_start(repo_root, port);
    assert!(!state.is_ready());
}

#[test]
fn get_read_later_empty_returns_200_array() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get_with_response(port, "/api/read-later");
        assert_eq!(status, 200);
        assert!(body.is_array());
        assert_eq!(body.as_array().expect("array").len(), 0);
    });
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
fn get_read_later_returns_entries_desc_by_saved_at() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        http_post(
            port,
            "/api/read-later",
            &json!({ "url": "https://example.com/1", "title": "first" }),
        );
        thread::sleep(Duration::from_millis(5));
        http_post(
            port,
            "/api/read-later",
            &json!({ "url": "https://example.com/2", "title": "second" }),
        );

        let (status, body) = http_get_with_response(port, "/api/read-later");
        assert_eq!(status, 200);
        let items = body.as_array().expect("array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["url"], "https://example.com/2");
        assert_eq!(items[1]["url"], "https://example.com/1");
    });
}

#[test]
fn patch_read_later_marks_entry_read() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (_, created) = http_post(
            port,
            "/api/read-later",
            &json!({ "url": "https://example.com/x", "title": "x" }),
        );
        let id = created["entry"]["id"].as_str().expect("id");

        let (status, body) =
            http_patch_with_response(port, &format!("/api/read-later/{id}"), &json!({ "read": true }));
        assert_eq!(status, 200);
        assert_eq!(body["entry"]["read"], true);
        assert_eq!(body["entry"]["id"], id);
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
fn patch_read_later_unknown_id_returns_404_with_cors() {
    let fixture = setup_repo_for_read_later();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_patch_with_response(
            port,
            "/api/read-later/00000000000000000000000000000000",
            &json!({ "read": true }),
        );
        assert_eq!(status, 404);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn patch_non_read_later_path_returns_405() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, _) =
            http_patch(port, "/api/status", &json!({ "read": true }));
        assert_eq!(status, 405);
    });
}

#[test]
fn options_non_read_later_path_returns_405() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, _) = http_options(port, "/api/status");
        assert_eq!(status, 405);
    });
}

#[test]
fn get_todo_tasks_empty_returns_200_array() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(status, 200);
        assert!(body.is_array());
        assert_eq!(body.as_array().expect("array").len(), 0);
    });
}

#[test]
fn get_todo_tasks_returns_created_masters() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Listed master", "sub_titles": ["Sub A", "Sub B"] }),
        );
        assert_eq!(create_status, 201);
        let master_id = create_body["master_task_id"]
            .as_str()
            .expect("master_task_id");

        let (status, body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(status, 200);
        let list = body.as_array().expect("array");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["master_task_id"], master_id);
        assert_eq!(list[0]["title"], "Listed master");
        let subs = list[0]["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 2);
    });
}

#[test]
fn post_todo_task_create_omit_sub_titles_creates_empty_sub_tasks() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Master only" }),
        );
        assert_eq!(status, 201);
        assert!(body.get("master_task_id").and_then(|v| v.as_str()).is_some());
        assert!(body.get("sub_task_id").is_none());
        let task = &body["task"];
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
        assert_eq!(task["status"], "incomplete");
    });
}

#[test]
fn post_todo_task_create_empty_sub_titles_matches_omit() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Empty array", "sub_titles": [] }),
        );
        assert_eq!(status, 201);
        let subs = body["task"]["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
        assert_eq!(body["task"]["status"], "incomplete");
    });
}

#[test]
fn post_todo_task_create_single_explicit_sub() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Master", "sub_titles": ["Sub A"] }),
        );
        assert_eq!(status, 201);
        let subs = body["task"]["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["implicit"], false);
        assert_eq!(subs[0]["title"], "Sub A");
    });
}

#[test]
fn post_todo_task_create_multiple_explicit_subs() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Master", "sub_titles": ["Sub A", "Sub B"] }),
        );
        assert_eq!(status, 201);
        let subs = body["task"]["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0]["implicit"], false);
        assert_eq!(subs[1]["implicit"], false);
        assert_eq!(subs[0]["title"], "Sub A");
        assert_eq!(subs[1]["title"], "Sub B");
    });
}

#[test]
fn post_todo_task_create_blank_title_returns_400() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "   " }),
        );
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn post_todo_task_create_response_ac5_field_matrix() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "AC5 master", "sub_titles": ["Sub A"] }),
        );
        assert_eq!(status, 201);
        assert!(body.get("master_task_id").and_then(|v| v.as_str()).is_some());
        assert!(body.get("sub_task_id").and_then(|v| v.as_str()).is_some());
        assert_ac5_master_task_shape(&body["task"]);
        assert_eq!(body["task"]["status"], "incomplete");
        assert_eq!(body["task"]["sub_tasks"][0]["title"], "Sub A");
        assert_eq!(body["task"]["sub_tasks"][0]["implicit"], false);

        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let master_id = body["master_task_id"].as_str().unwrap();
        assert!(
            wb.join("todo_tasks")
                .join("tasks")
                .join(master_id)
                .join("sub_tasks.json")
                .is_file(),
            "v2 storage must exist but remain invisible in HTTP shape"
        );
    });
}

fn create_todo_master(port: u16, title: &str, sub_titles: &[&str]) -> (String, String, Value) {
    let payload = if sub_titles.is_empty() {
        json!({ "title": title })
    } else {
        json!({ "title": title, "sub_titles": sub_titles })
    };
    let (status, body) = http_post(port, "/api/todo-task-create", &payload);
    assert_eq!(status, 201);
    let master_id = body["master_task_id"].as_str().expect("master_task_id").to_string();
    let sub_id = body["sub_task_id"]
        .as_str()
        .or_else(|| {
            body["task"]["sub_tasks"]
                .as_array()
                .and_then(|subs| subs.first())
                .and_then(|sub| sub["sub_task_id"].as_str())
        })
        .expect("sub_task_id")
        .to_string();
    (master_id, sub_id, body)
}

#[test]
fn todo_task_master_status_wire_includes_abandoned() {
    assert_eq!(
        TODO_TASK_MASTER_STATUS_WIRE,
        &["incomplete", "complete", "abandoned"]
    );
}

#[test]
fn http_list_get_create_carry_tri_state_status_including_abandoned() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Create incomplete", "sub_titles": ["Sub"] }),
        );
        assert_eq!(create_status, 201);
        assert_ac5_master_task_shape(&create_body["task"]);
        assert_eq!(create_body["task"]["status"], "incomplete");

        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let complete_id = "task_http_status_complete";
        let abandoned_id = "task_http_status_abandoned";
        seed_v2_todo_for_http(
            &wb,
            complete_id,
            "Stored complete",
            "complete",
            &json!({ "sub_tasks": [] }),
            true,
        );
        seed_v2_todo_for_http(
            &wb,
            abandoned_id,
            "Stored abandoned",
            "abandoned",
            &json!({ "sub_tasks": [] }),
            true,
        );

        let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(list_status, 200);
        let list = list_body.as_array().expect("array");
        let complete = list
            .iter()
            .find(|t| t["master_task_id"] == complete_id)
            .expect("complete in list");
        let abandoned = list
            .iter()
            .find(|t| t["master_task_id"] == abandoned_id)
            .expect("abandoned in list");
        assert_eq!(complete["status"], "complete");
        assert_eq!(abandoned["status"], "abandoned");
        assert!(TODO_TASK_MASTER_STATUS_WIRE.contains(&complete["status"].as_str().unwrap()));
        assert!(TODO_TASK_MASTER_STATUS_WIRE.contains(&abandoned["status"].as_str().unwrap()));

        let (get_complete_status, get_complete) =
            http_get(port, &format!("/api/todo-task?id={complete_id}"));
        assert_eq!(get_complete_status, 200);
        assert_eq!(get_complete["status"], "complete");

        let (get_abandoned_status, get_abandoned) =
            http_get(port, &format!("/api/todo-task?id={abandoned_id}"));
        assert_eq!(get_abandoned_status, 200);
        assert_eq!(get_abandoned["status"], "abandoned");
    });
}

#[test]
fn post_todo_task_set_status_updates_master_and_reads_back_consistently() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, _, _) = create_todo_master(port, "Set status master", &["Sub"]);
        assert_eq!(
            http_get(port, &format!("/api/todo-task?id={master_id}")).1["status"],
            "incomplete"
        );

        for target in ["complete", "abandoned", "incomplete"] {
            let (status, body) = http_post(
                port,
                "/api/todo-task-set-status",
                &json!({ "master_task_id": master_id, "status": target }),
            );
            assert_eq!(status, 200, "set-status {target}: {body}");
            assert_eq!(body["task"]["status"], target);
            assert_eq!(body["task"]["master_task_id"], master_id);

            let (get_status, get_body) =
                http_get(port, &format!("/api/todo-task?id={master_id}"));
            assert_eq!(get_status, 200);
            assert_eq!(get_body["status"], target);

            let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
            assert_eq!(list_status, 200);
            let listed = list_body
                .as_array()
                .expect("list array")
                .iter()
                .find(|t| t["master_task_id"] == master_id)
                .expect("master in list");
            assert_eq!(listed["status"], target);

            let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
            let index: Value = serde_json::from_str(
                &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(index["tasks"][&master_id]["status"], target);
            assert_eq!(index["tasks"][&master_id]["status"], get_body["status"]);
            assert_eq!(index["tasks"][&master_id]["status"], listed["status"]);
        }
    });
}

#[test]
fn post_todo_task_set_status_rejects_invalid_status() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, _, _) = create_todo_master(port, "Invalid status", &["Sub"]);
        let (status, body) = http_post(
            port,
            "/api/todo-task-set-status",
            &json!({ "master_task_id": master_id, "status": "done" }),
        );
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn todo_task_crud_http_flow() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, sub_a, create_body) =
            create_todo_master(port, "CRUD master", &["Sub A", "Sub B"]);
        assert_eq!(create_body["task"]["sub_tasks"].as_array().unwrap().len(), 2);

        let (get_status, get_body) =
            http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(get_status, 200);
        assert_eq!(get_body["master_task_id"], master_id);
        assert_eq!(get_body["title"], "CRUD master");
        assert!(get_body.get("_status").is_none());

        let (add_status, add_body) = http_post(
            port,
            "/api/todo-task-add-sub",
            &json!({ "master_task_id": master_id, "title": "Sub C" }),
        );
        assert_eq!(add_status, 201);
        let sub_c = add_body["sub_task_id"].as_str().expect("sub_task_id");
        assert_eq!(add_body["task"]["sub_tasks"].as_array().unwrap().len(), 3);

        let (complete_status, complete_body) = http_post(
            port,
            "/api/todo-task-complete",
            &json!({ "master_task_id": master_id, "sub_task_id": sub_a }),
        );
        assert_eq!(complete_status, 200);
        assert_eq!(complete_body["task"]["sub_tasks"][0]["status"], "complete");
        assert_eq!(complete_body["task"]["status"], "incomplete");
        assert!(complete_body["task"]["sub_tasks"][0]
            .get("completed_at")
            .and_then(|v| v.as_str())
            .is_some());

        let (link_status, link_body) = http_post(
            port,
            "/api/todo-task-link-archive",
            &json!({
                "master_task_id": master_id,
                "sub_task_id": sub_a,
                "archive_id": "11111111111111111111111111111111",
            }),
        );
        assert_eq!(link_status, 200);
        let linked = link_body["task"]["sub_tasks"][0]["linked_archive_ids"]
            .as_array()
            .expect("linked_archive_ids");
        assert_eq!(linked.len(), 1);
        assert_eq!(linked[0], "11111111111111111111111111111111");

        let (del_sub_status, del_sub_body) = http_post(
            port,
            "/api/todo-task-delete-sub",
            &json!({ "master_task_id": master_id, "sub_task_id": sub_c }),
        );
        assert_eq!(del_sub_status, 200);
        assert_eq!(del_sub_body["task"]["sub_tasks"].as_array().unwrap().len(), 2);

        let (delete_status, delete_body) = http_post(
            port,
            "/api/todo-task-delete",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(delete_status, 200);
        assert_eq!(delete_body["ok"], true);
        assert!(delete_body.get("_status").is_none());

        let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(list_status, 200);
        assert_eq!(list_body.as_array().unwrap().len(), 0);
    });
}

#[test]
fn get_todo_task_missing_id_returns_400() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/todo-task");
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn get_todo_task_unknown_id_returns_404() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) =
            http_get(port, "/api/todo-task?id=00000000000000000000000000000000");
        assert_eq!(status, 404);
        assert_eq!(body["error"], "Task not found");
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn post_todo_task_add_sub_missing_or_blank_title_returns_400() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, _, _) = create_todo_master(port, "Master", &["Sub A"]);

        for payload in [
            json!({ "master_task_id": master_id }),
            json!({ "master_task_id": master_id, "title": "" }),
            json!({ "master_task_id": master_id, "title": "   " }),
        ] {
            let (status, body) = http_post(port, "/api/todo-task-add-sub", &payload);
            assert_eq!(status, 400);
            assert!(body.get("error").is_some());
            assert!(body.get("_status").is_none());
        }
    });
}

#[test]
fn post_todo_task_delete_sub_last_sub_allows_empty() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, sub_id, _) = create_todo_master(port, "Single sub", &["Single sub"]);
        let (status, body) = http_post(
            port,
            "/api/todo-task-delete-sub",
            &json!({ "master_task_id": master_id, "sub_task_id": sub_id }),
        );
        assert_eq!(status, 200);
        assert!(body.get("_status").is_none());
        assert!(body["task"]["sub_tasks"].as_array().unwrap().is_empty());
        assert_eq!(body["task"]["status"], "incomplete");
    });
}

#[test]
fn post_todo_task_unknown_master_returns_404() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let unknown = "00000000000000000000000000000000";
        for path in [
            "/api/todo-task-delete",
            "/api/todo-task-add-sub",
            "/api/todo-task-delete-sub",
            "/api/todo-task-complete",
            "/api/todo-task-link-archive",
        ] {
            let payload = match path {
                "/api/todo-task-delete" => json!({ "master_task_id": unknown }),
                "/api/todo-task-add-sub" => {
                    json!({ "master_task_id": unknown, "title": "Sub" })
                }
                "/api/todo-task-delete-sub" | "/api/todo-task-complete" => {
                    json!({
                        "master_task_id": unknown,
                        "sub_task_id": "00000000000000000000000000000001",
                    })
                }
                "/api/todo-task-link-archive" => {
                    json!({
                        "master_task_id": unknown,
                        "sub_task_id": "00000000000000000000000000000001",
                        "archive_id": "11111111111111111111111111111111",
                    })
                }
                _ => unreachable!(),
            };
            let (status, body) = http_post(port, path, &payload);
            assert_eq!(status, 404, "expected 404 for {path}");
            assert_eq!(body["error"], "Task not found");
            assert!(body.get("_status").is_none());
        }
    });
}

#[test]
fn post_todo_task_unknown_sub_returns_404() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, _, _) = create_todo_master(port, "Master", &["Sub A", "Sub B"]);
        let unknown_sub = "00000000000000000000000000000099";
        for path in [
            "/api/todo-task-delete-sub",
            "/api/todo-task-complete",
            "/api/todo-task-link-archive",
        ] {
            let payload = match path {
                "/api/todo-task-delete-sub" | "/api/todo-task-complete" => {
                    json!({ "master_task_id": master_id, "sub_task_id": unknown_sub })
                }
                "/api/todo-task-link-archive" => {
                    json!({
                        "master_task_id": master_id,
                        "sub_task_id": unknown_sub,
                        "archive_id": "11111111111111111111111111111111",
                    })
                }
                _ => unreachable!(),
            };
            let (status, body) = http_post(port, path, &payload);
            assert_eq!(status, 404, "expected 404 for {path}");
            assert_eq!(body["error"], "Task not found");
            assert!(body.get("_status").is_none());
        }
    });
}

#[test]
fn old_plan_http_paths_unavailable_and_complete_sub_unmapped() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, sub_id, _) = create_todo_master(port, "Hard cut", &["Sub A"]);

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

#[test]
fn post_todo_task_complete_without_sub_marks_master_complete() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, _, _) = create_todo_master(port, "Master only", &["Sub A"]);
        let (status, body) = http_post(
            port,
            "/api/todo-task-complete",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(status, 200);
        assert!(body.get("_status").is_none());
        assert_eq!(body["task"]["status"], "complete");
        assert_eq!(body["task"]["sub_tasks"][0]["status"], "incomplete");
    });
}

#[test]
fn post_todo_task_create_blank_sub_title_element_returns_400() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Master", "sub_titles": ["ok", "  "] }),
        );
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

fn seed_v2_todo_for_http(
    wb: &std::path::Path,
    master_id: &str,
    title: &str,
    status: &str,
    sub_tasks: &Value,
    merge_index: bool,
) {
    let plan_tasks_dir = wb.join("todo_tasks");
    fs::create_dir_all(plan_tasks_dir.join("tasks").join(master_id)).expect("mkdir task");
    if merge_index {
        let index_path = plan_tasks_dir.join("index.json");
        let mut index: Value = if index_path.is_file() {
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap_or_else(|_| {
                json!({ "version": 2, "tasks": {} })
            })
        } else {
            fs::create_dir_all(plan_tasks_dir.join("tasks")).expect("mkdir tasks");
            json!({ "version": 2, "tasks": {} })
        };
        index["tasks"][master_id] = json!({
            "master_task_id": master_id,
            "title": title,
            "status": status,
            "created_at": "2026-07-08T00:00:00+00:00",
            "task_dir": format!("tasks/{master_id}")
        });
        fs::write(
            index_path,
            serde_json::to_string_pretty(&index).expect("serialize index"),
        )
        .expect("write index");
    }
    fs::write(
        plan_tasks_dir
            .join("tasks")
            .join(master_id)
            .join("sub_tasks.json"),
        serde_json::to_string_pretty(sub_tasks).expect("serialize subs"),
    )
    .expect("write sub_tasks");
    fs::write(
        plan_tasks_dir
            .join("tasks")
            .join(master_id)
            .join("todo.md"),
        "",
    )
    .expect("write plan.md");
}

fn assert_http_master_has_todo_fields(task: &Value) {
    assert!(
        task.get("todo_md").is_some(),
        "todo_md must be a top-level field"
    );
    assert!(
        task.get("migration_error").and_then(|v| v.as_bool()).is_some(),
        "migration_error must be a top-level boolean"
    );
}

#[test]
fn get_todo_tasks_includes_todo_md_and_migration_error_fields() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, _) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "HTTP plan fields" }),
        );
        assert_eq!(create_status, 201);

        let (status, body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(status, 200);
        let list = body.as_array().expect("array");
        assert_eq!(list.len(), 1);
        assert_http_master_has_todo_fields(&list[0]);
        assert_eq!(list[0]["todo_md"], "");
        assert_eq!(list[0]["migration_error"], false);

        let expected = crate::services::todo_task::list_all();
        assert_eq!(body, expected);
    });
}

#[test]
fn get_todo_task_by_id_includes_todo_md_and_migration_error_matching_list() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "HTTP get by id fields" }),
        );
        assert_eq!(create_status, 201);
        let master_id = create_body["master_task_id"]
            .as_str()
            .expect("master_task_id");

        let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(list_status, 200);
        let list = list_body.as_array().expect("array");
        assert_eq!(list.len(), 1);

        let (get_status, get_body) =
            http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(get_status, 200);
        assert_http_master_has_todo_fields(&get_body);
        assert_eq!(get_body["todo_md"], list[0]["todo_md"]);
        assert_eq!(get_body["migration_error"], list[0]["migration_error"]);
        assert!(get_body.get("_status").is_none());

        let expected = crate::services::todo_task::get_by_id(master_id);
        assert_eq!(get_body, expected);
    });
}

#[test]
fn get_todo_tasks_todo_md_matches_disk_bytes() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Disk plan md" }),
        );
        assert_eq!(create_status, 201);
        let master_id = create_body["master_task_id"]
            .as_str()
            .expect("master_task_id");

        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let plan_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(master_id)
            .join("todo.md");
        let content = "# Title\n\n## Section\n\n- item one\n";
        fs::write(&plan_path, content).expect("write plan.md");

        let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(list_status, 200);
        assert_eq!(list_body[0]["todo_md"], content);
        assert_eq!(fs::read_to_string(&plan_path).unwrap(), content);

        let (get_status, get_body) =
            http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(get_status, 200);
        assert_eq!(get_body["todo_md"], content);
    });
}

#[test]
fn get_todo_tasks_empty_todo_md_matches_empty_disk_file() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Empty plan md" }),
        );
        assert_eq!(create_status, 201);
        let master_id = create_body["master_task_id"]
            .as_str()
            .expect("master_task_id");

        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let plan_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(master_id)
            .join("todo.md");
        assert_eq!(fs::read_to_string(&plan_path).unwrap(), "");

        let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(list_status, 200);
        assert_eq!(list_body[0]["todo_md"], "");
        assert_eq!(list_body[0]["migration_error"], false);
    });
}

#[test]
fn todo_tasks_http_body_helpers_preserve_read_path_fields() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Body helper fields" }),
        );
        assert_eq!(create_status, 201);
        let master_id = create_body["master_task_id"]
            .as_str()
            .expect("master_task_id");

        let list_value = crate::services::todo_task::list_all();
        let list_body = todo_tasks_list_response_body(&list_value);
        let list_parsed: Value = serde_json::from_str(&list_body).expect("list json");
        assert_eq!(list_parsed, list_value);
        assert_http_master_has_todo_fields(&list_parsed[0]);

        let get_value = crate::services::todo_task::get_by_id(master_id);
        let (get_status, get_body) = todo_task_get_response_body(&get_value);
        assert_eq!(get_status, 200);
        let get_parsed: Value = serde_json::from_str(&get_body).expect("get json");
        assert_eq!(get_parsed, get_value);
        assert_http_master_has_todo_fields(&get_parsed);
    });
}

#[test]
fn post_todo_task_create_with_todo_md_persists_and_lists() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let content = "## HTTP create\n\nBody";
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Create with md", "todo_md": content }),
        );
        assert_eq!(create_status, 201);
        let master_id = create_body["master_task_id"]
            .as_str()
            .expect("master_task_id");

        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let plan_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(master_id)
            .join("todo.md");
        assert_eq!(fs::read_to_string(&plan_path).unwrap(), content);

        let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(list_status, 200);
        assert_eq!(list_body[0]["todo_md"], content);

        let (get_status, get_body) =
            http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(get_status, 200);
        assert_eq!(get_body["todo_md"], content);
    });
}

#[test]
fn post_todo_task_create_title_too_long_returns_400() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({
                "title": "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone"
            }),
        );
        assert_eq!(status, 400);
        assert!(
            body["error"]
                .as_str()
                .unwrap_or("")
                .contains("Title too long"),
            "error={:?}",
            body["error"]
        );
    });
}

#[test]
fn get_todo_tasks_migration_error_plan_still_in_list() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let master_id = "task_http_migrate_err";
        seed_v2_todo_for_http(
            &wb,
            master_id,
            "Migration error plan",
            "incomplete",
            &json!({ "sub_tasks": [] }),
            true,
        );
        fs::write(
            wb.join("todo_tasks")
                .join("tasks")
                .join(master_id)
                .join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let (status, body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(status, 200);
        let list = body.as_array().expect("array");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["master_task_id"], master_id);
        assert_eq!(list[0]["migration_error"], true);
        assert_http_master_has_todo_fields(&list[0]);

        let (get_status, get_body) =
            http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(get_status, 200);
        assert_eq!(get_body["migration_error"], true);
        assert_eq!(get_body["todo_md"], "");
    });
}

fn create_todo_master_id(port: u16, title: &str) -> String {
    let (status, body) = http_post(port, "/api/todo-task-create", &json!({ "title": title }));
    assert_eq!(status, 201);
    body["master_task_id"]
        .as_str()
        .expect("master_task_id")
        .to_string()
}


fn write_http_attach_source(name: &str, content: impl AsRef<[u8]>) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "todo_attach_http_{}_{}",
        std::process::id(),
        n
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path.canonicalize().unwrap_or(path)
}

#[test]
fn todo_task_attachment_http_flow() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Attach HTTP");

        let add_src = write_http_attach_source("notes.md", "# Notes\n");
        let add_payload = json!({
            "master_task_id": master_id,
            "source_path": add_src.to_str().unwrap(),
        });
        let (add_status, add_body) =
            http_post(port, "/api/todo-task-add-attachment", &add_payload);
        assert_eq!(add_status, 201);
        assert_eq!(add_body["file_name"], "notes.md");
        assert_eq!(add_body["original_file_name"], "notes.md");
        assert!(add_body.get("added_at").and_then(|v| v.as_str()).is_some());
        assert!(add_body.get("_status").is_none());

        let (list_status, list_body) = http_post(
            port,
            "/api/todo-task-list-attachments",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(list_status, 200);
        let attachments = list_body["attachments"].as_array().expect("attachments");
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0]["file_name"], "notes.md");
        assert!(list_body.get("_status").is_none());

        let (get_status, get_body) = http_post(
            port,
            "/api/todo-task-get-attachment",
            &json!({
                "master_task_id": master_id,
                "file_name": "notes.md",
            }),
        );
        assert_eq!(get_status, 200);
        assert_eq!(get_body["file_name"], "notes.md");
        assert_eq!(get_body["content"], "# Notes\n");
        assert!(get_body.get("_status").is_none());

        let update_src = write_http_attach_source("notes.md", "updated body");
        let update_payload = json!({
            "master_task_id": master_id,
            "file_name": "notes.md",
            "source_path": update_src.to_str().unwrap(),
        });
        let (update_status, update_body) =
            http_post(port, "/api/todo-task-update-attachment", &update_payload);
        assert_eq!(update_status, 200);
        assert_eq!(update_body["ok"], true);
        assert!(update_body.get("_status").is_none());

        let (reread_status, reread_body) = http_post(
            port,
            "/api/todo-task-get-attachment",
            &json!({
                "master_task_id": master_id,
                "file_name": "notes.md",
            }),
        );
        assert_eq!(reread_status, 200);
        assert_eq!(reread_body["content"], "updated body");
    });
}

#[test]
fn todo_task_update_http_title_and_body() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "HTTP Update", "todo_md": "old body" }),
        );
        assert_eq!(create_status, 201);
        let master_id = create_body["master_task_id"].as_str().unwrap();

        let (title_status, title_body) = http_post(
            port,
            "/api/todo-task-update",
            &json!({
                "master_task_id": master_id,
                "title": "Renamed HTTP",
            }),
        );
        assert_eq!(title_status, 200);
        assert_eq!(title_body["task"]["title"], "Renamed HTTP");
        assert_eq!(title_body["task"]["todo_md"], "old body");
        assert!(title_body.get("_status").is_none());

        let (body_status, body_body) = http_post(
            port,
            "/api/todo-task-update",
            &json!({
                "master_task_id": master_id,
                "todo_md": "new body",
            }),
        );
        assert_eq!(body_status, 200);
        assert_eq!(body_body["task"]["title"], "Renamed HTTP");
        assert_eq!(body_body["task"]["todo_md"], "new body");

        let (both_status, both_body) = http_post(
            port,
            "/api/todo-task-update",
            &json!({
                "master_task_id": master_id,
                "title": "Both",
                "todo_md": "both body",
            }),
        );
        assert_eq!(both_status, 200);
        assert_eq!(both_body["task"]["title"], "Both");
        assert_eq!(both_body["task"]["todo_md"], "both body");

        let (missing_status, missing_body) = http_post(
            port,
            "/api/todo-task-update",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(missing_status, 400);
        assert_eq!(missing_body["error"], "Missing title or todo_md");

        let (get_status, get_body) =
            http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(get_status, 200);
        assert_eq!(get_body["title"], "Both");
        assert_eq!(get_body["todo_md"], "both body");
    });
}

#[test]
fn todo_task_attachment_http_paths_follow_todo_task_verb_prefix() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Path style");
        for path in [
            "/api/todo-task-add-attachment",
            "/api/todo-task-list-attachments",
            "/api/todo-task-get-attachment",
            "/api/todo-task-update-attachment",
        ] {
            assert!(
                path.starts_with("/api/todo-task-"),
                "path {path} must use todo-task prefix"
            );
        }

        let add_src = write_http_attach_source("a.md", "x");
        let add_payload = json!({
            "master_task_id": master_id,
            "source_path": add_src.to_str().unwrap(),
        });
        let (status, body) = http_post(port, "/api/todo-task-add-attachment", &add_payload);
        assert_eq!(status, 201, "add-attachment must be registered: {body}");
        assert!(body.get("error").is_none());
    });
}

#[test]
fn todo_task_attachment_http_has_no_delete_or_ui_paths() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "No delete HTTP");
        let add_src = write_http_attach_source("notes.md", "keep");
        let add_payload = json!({
            "master_task_id": master_id,
            "source_path": add_src.to_str().unwrap(),
        });
        let (add_status, _) = http_post(port, "/api/todo-task-add-attachment", &add_payload);
        assert_eq!(add_status, 201);

        for path in [
            "/api/todo-task-delete-attachment",
            "/api/todo-task-remove-attachment",
            "/api/todo-task-ui-add-attachment",
            "/api/todo-task-open-attachment-dialog",
        ] {
            let (status, body) = http_post(
                port,
                path,
                &json!({
                    "master_task_id": master_id,
                    "file_name": "notes.md",
                }),
            );
            // Unregistered POST paths fall through to Method not allowed (not a real handler).
            assert_eq!(status, 405, "expected no HTTP path {path}");
            assert_eq!(body["error"], "Method not allowed");
        }

        let (list_status, list_body) = http_post(
            port,
            "/api/todo-task-list-attachments",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(list_status, 200);
        assert_eq!(list_body["attachments"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn todo_api_gate_missing_marker_keeps_todo_http_gated_while_status_ok() {
    let fixture = setup_repo_for_todo_task_without_gate();
    let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status_ok, status_body) = http_get(port, "/api/status");
        assert_eq!(status_ok, 200, "App may start without migration gate");
        assert_eq!(status_body["ok"], true);

        let (list_status, list_body) = http_get(port, "/api/todo-tasks");
        assert_eq!(
            list_status, 503,
            "missing .migration_gate_passed must gate GET /api/todo-tasks"
        );
        assert!(
            list_body.get("error").and_then(|v| v.as_str()).is_some(),
            "gated response must use Host error envelope"
        );

        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "must stay gated" }),
        );
        assert_eq!(
            create_status, 503,
            "missing marker must gate POST /api/todo-task-create (falsifier: open write)"
        );
        assert!(create_body.get("error").is_some());
    });
    assert!(
        !wb.join("todo_tasks").join(MIGRATION_GATE_FILE).is_file(),
        "startup/request must not silently write the durable gate marker"
    );
}

#[test]
fn todo_api_gate_missing_marker_does_not_auto_migrate_on_startup() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let plan_root = wb.join("plan_tasks");
    fs::create_dir_all(plan_root.join("tasks")).expect("mkdir plan_tasks/tasks");
    fs::write(
        plan_root.join("index.json"),
        br#"{"version":2,"tasks":{}}"#,
    )
    .expect("write plan index");
    let repo_root = sandbox.config_dir().to_path_buf();
    with_server(repo_root, |port| {
        let (st, _) = http_get(port, "/api/status");
        assert_eq!(st, 200);
        let (list_status, _) = http_get(port, "/api/todo-tasks");
        assert_eq!(list_status, 503);
    });
    assert!(
        plan_root.is_dir(),
        "Host must not silently migrate plan_tasks/ on startup"
    );
    assert!(
        !wb.join("todo_tasks").join(MIGRATION_GATE_FILE).is_file(),
        "no silent gate write"
    );
    assert!(
        !wb.join("todo_tasks").join("index.json").is_file(),
        "no silent plan_tasks→todo_tasks migration on startup"
    );
}

#[test]
fn todo_api_gate_passed_marker_opens_todo_http() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(status, 200);
        assert!(body.is_array());
        let (create_status, create_body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "ungated master" }),
        );
        assert_eq!(create_status, 201);
        assert!(create_body.get("master_task_id").is_some());
    });
}

#[test]
fn todo_api_gate_cold_restart_rereads_durable_marker_not_process_exit_code() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(&wb).expect("mkdir wb");
    let repo_root = sandbox.config_dir().to_path_buf();

    // Cold start A: no durable marker → gated (must not depend on in-process script exit).
    with_server(repo_root.clone(), |port| {
        let (status, body) = http_get(port, "/api/todo-tasks");
        assert_eq!(status, 503);
        assert!(body.get("error").is_some());
    });

    // External operator success path (t5): durable marker appears on disk.
    plant_migration_gate(&wb);

    // Cold start B: new server must re-read the marker from disk.
    with_server(repo_root, |port| {
        let (status, body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(
            status, 200,
            "cold restart must open todo API after durable marker appears"
        );
        assert!(body.is_array());
    });
}

#[test]
fn todo_api_gate_rereads_marker_while_server_running() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    fs::create_dir_all(&wb).expect("mkdir wb");
    let repo_root = sandbox.config_dir().to_path_buf();
    with_server(repo_root, |port| {
        let (before, _) = http_get(port, "/api/todo-tasks");
        assert_eq!(before, 503);
        plant_migration_gate(&wb);
        let (after, body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(
            after, 200,
            "gate check must re-read durable marker per request, not cache a process-local flag"
        );
        assert!(body.is_array());
    });
}

#[test]
fn todo_task_comment_http_four_routes_registered() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Comment routes");
        for path in [
            "/api/todo-task-list-comments",
            "/api/todo-task-add-comment",
            "/api/todo-task-update-comment",
            "/api/todo-task-delete-comment",
        ] {
            assert!(
                path.starts_with("/api/todo-task-"),
                "path {path} must use todo-task prefix"
            );
        }

        let (list_status, list_body) = http_post(
            port,
            "/api/todo-task-list-comments",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(list_status, 200, "list-comments must be registered: {list_body}");
        assert!(list_body.get("comments").and_then(|v| v.as_array()).is_some());
        assert!(list_body.get("error").is_none());

        let (add_status, add_body) = http_post(
            port,
            "/api/todo-task-add-comment",
            &json!({ "master_task_id": master_id, "body": "route probe" }),
        );
        assert_eq!(add_status, 201, "add-comment must be registered: {add_body}");
        assert!(add_body.get("error").is_none());
        let comment_id = add_body["id"].as_str().expect("comment id");

        let (update_status, update_body) = http_post(
            port,
            "/api/todo-task-update-comment",
            &json!({
                "master_task_id": master_id,
                "comment_id": comment_id,
                "body": "updated probe",
            }),
        );
        assert_eq!(
            update_status, 200,
            "update-comment must be registered: {update_body}"
        );
        assert!(update_body.get("error").is_none());

        let (delete_status, delete_body) = http_post(
            port,
            "/api/todo-task-delete-comment",
            &json!({
                "master_task_id": master_id,
                "comment_id": comment_id,
            }),
        );
        assert_eq!(
            delete_status, 200,
            "delete-comment must be registered: {delete_body}"
        );
        assert_eq!(delete_body["ok"], true);
    });
}

#[test]
fn todo_task_comment_http_flow_list_envelope() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Comment HTTP");

        let (empty_status, empty_body) = http_post(
            port,
            "/api/todo-task-list-comments",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(empty_status, 200);
        let empty = empty_body["comments"].as_array().expect("comments envelope");
        assert!(empty.is_empty());
        assert!(empty_body.get("_status").is_none());

        let (add_status, add_body) = http_post(
            port,
            "/api/todo-task-add-comment",
            &json!({
                "master_task_id": master_id,
                "body": "first note",
            }),
        );
        assert_eq!(add_status, 201);
        assert_eq!(add_body["body"], "first note");
        assert!(add_body.get("id").and_then(|v| v.as_str()).is_some());
        assert!(add_body.get("created_at").and_then(|v| v.as_str()).is_some());
        assert!(add_body.get("_status").is_none());
        let comment_id = add_body["id"].as_str().expect("id").to_string();

        let (list_status, list_body) = http_post(
            port,
            "/api/todo-task-list-comments",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(list_status, 200);
        let comments = list_body["comments"].as_array().expect("comments");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0]["id"], comment_id);
        assert_eq!(comments[0]["body"], "first note");
        // File-root-shaped envelope: object with comments array, not a bare array.
        assert!(list_body.is_object());
        assert!(!list_body.is_array());

        let (update_status, update_body) = http_post(
            port,
            "/api/todo-task-update-comment",
            &json!({
                "master_task_id": master_id,
                "comment_id": comment_id,
                "body": "edited note",
            }),
        );
        assert_eq!(update_status, 200);
        assert_eq!(update_body["id"], comment_id);
        assert_eq!(update_body["body"], "edited note");

        let (reread_status, reread_body) = http_post(
            port,
            "/api/todo-task-list-comments",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(reread_status, 200);
        assert_eq!(reread_body["comments"][0]["body"], "edited note");

        let (delete_status, delete_body) = http_post(
            port,
            "/api/todo-task-delete-comment",
            &json!({
                "master_task_id": master_id,
                "comment_id": comment_id,
            }),
        );
        assert_eq!(delete_status, 200);
        assert_eq!(delete_body["ok"], true);

        let (after_status, after_body) = http_post(
            port,
            "/api/todo-task-list-comments",
            &json!({ "master_task_id": master_id }),
        );
        assert_eq!(after_status, 200);
        assert_eq!(after_body["comments"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn todo_task_comment_http_has_no_get_by_id() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "No get-by-id");
        let comment_id = "cmt_probe00000001";
        let get_query_path = format!("/api/todo-task-comment?id={comment_id}");

        for path in [
            "/api/todo-task-get-comment",
            "/api/todo-task-read-comment",
            get_query_path.as_str(),
        ] {
            let (status, body) = if path.contains('?') {
                http_get(port, path)
            } else {
                http_post(
                    port,
                    path,
                    &json!({
                        "master_task_id": master_id,
                        "comment_id": comment_id,
                    }),
                )
            };
            assert!(
                status == 404 || status == 405,
                "get-by-id HTTP must not be registered, got {status} for {path}: {body}"
            );
            assert!(body.get("error").is_some());
            assert_ne!(status, 200);
            assert_ne!(status, 201);
        }
    });
}

#[test]
fn todo_task_comment_unregistered_paths_are_not_success_handlers() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Unregistered comment paths");
        for path in [
            "/api/todo-task-comments",
            "/api/todo-task-list-comment",
            "/api/todo-task-add-comments",
            "/api/plan-task-list-comments",
            "/api/todo-task-ui-add-comment",
        ] {
            let (status, body) = http_post(
                port,
                path,
                &json!({
                    "master_task_id": master_id,
                    "body": "should not match",
                    "comment_id": "cmt_x",
                }),
            );
            assert!(
                status == 404 || status == 405,
                "unregistered path must not succeed, got {status} for {path}: {body}"
            );
            assert!(body.get("error").is_some());
            assert_ne!(status, 200);
            assert_ne!(status, 201);
        }
    });
}

#[test]
fn todo_task_comment_http_surfaces_service_errors() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Bad comments file");

        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let comments_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(&master_id)
            .join("comments.json");
        fs::write(&comments_path, "{not valid json").expect("write corrupt comments");

        let (status, body) = http_post(
            port,
            "/api/todo-task-list-comments",
            &json!({ "master_task_id": master_id }),
        );
        assert!(
            status >= 400,
            "corrupt comments.json must not look like success, got {status}: {body}"
        );
        assert!(
            body.get("error").is_some(),
            "service error must be visible on HTTP face"
        );
        // Must not masquerade as a successful empty list.
        assert_ne!(status, 200);
        assert_ne!(status, 201);

        let (unknown_status, unknown_body) = http_post(
            port,
            "/api/todo-task-list-comments",
            &json!({ "master_task_id": "00000000000000000000000000000000" }),
        );
        assert_eq!(unknown_status, 404);
        assert_eq!(unknown_body["error"], "Task not found");
    });
}

#[test]
fn post_todo_task_add_sub_title_only_without_content_field_returns_201() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Title only add");
        let (status, body) = http_post(
            port,
            "/api/todo-task-add-sub",
            &json!({ "master_task_id": master_id, "title": "Just title" }),
        );
        assert_eq!(status, 201);
        assert!(body.get("_status").is_none());
        let subs = body["task"]["sub_tasks"].as_array().expect("subs");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["title"], "Just title");
        assert!(subs[0].get("content").is_none() || subs[0]["content"].is_null());
    });
}

#[test]
fn post_todo_task_add_sub_with_optional_content_persists_via_get() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Add with content");
        let (add_status, add_body) = http_post(
            port,
            "/api/todo-task-add-sub",
            &json!({
                "master_task_id": master_id,
                "title": "With body",
                "content": "http content",
            }),
        );
        assert_eq!(add_status, 201);
        assert_eq!(add_body["task"]["sub_tasks"][0]["content"], "http content");

        let (get_status, get_body) =
            http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(get_status, 200);
        assert_eq!(get_body["sub_tasks"][0]["content"], "http content");

        let (list_status, list_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(list_status, 200);
        let listed = list_body
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("listed");
        assert_eq!(listed["sub_tasks"][0]["content"], "http content");
    });
}

#[test]
fn post_todo_task_update_sub_writes_clears_and_omits_content() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let master_id = create_todo_master_id(port, "Update sub content");
        let (add_status, add_body) = http_post(
            port,
            "/api/todo-task-add-sub",
            &json!({
                "master_task_id": master_id,
                "title": "Sub",
                "content": "v1",
            }),
        );
        assert_eq!(add_status, 201);
        let sub_id = add_body["sub_task_id"].as_str().expect("sub_task_id").to_string();

        let (set_status, set_body) = http_post(
            port,
            "/api/todo-task-update-sub",
            &json!({
                "master_task_id": master_id,
                "sub_task_id": sub_id,
                "title": "Sub",
                "content": "v2",
            }),
        );
        assert_eq!(set_status, 200);
        assert_eq!(set_body["task"]["sub_tasks"][0]["content"], "v2");

        let (omit_status, omit_body) = http_post(
            port,
            "/api/todo-task-update-sub",
            &json!({
                "master_task_id": master_id,
                "sub_task_id": sub_id,
                "title": "Renamed",
            }),
        );
        assert_eq!(omit_status, 200);
        assert_eq!(omit_body["task"]["sub_tasks"][0]["title"], "Renamed");
        assert_eq!(omit_body["task"]["sub_tasks"][0]["content"], "v2");

        let (clear_status, clear_body) = http_post(
            port,
            "/api/todo-task-update-sub",
            &json!({
                "master_task_id": master_id,
                "sub_task_id": sub_id,
                "title": "Renamed",
                "content": "",
            }),
        );
        assert_eq!(clear_status, 200);
        let sub = &clear_body["task"]["sub_tasks"][0];
        assert!(sub.get("content").is_none() || sub["content"].is_null() || sub["content"] == "");
    });
}

#[test]
fn post_todo_task_update_sub_missing_or_blank_title_returns_400() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, sub_id, _) = create_todo_master(port, "Update blank", &["A"]);
        for payload in [
            json!({ "master_task_id": master_id, "sub_task_id": sub_id }),
            json!({
                "master_task_id": master_id,
                "sub_task_id": sub_id,
                "title": "",
                "content": "ignored",
            }),
            json!({
                "master_task_id": master_id,
                "sub_task_id": sub_id,
                "title": "   ",
                "content": "ignored",
            }),
        ] {
            let (status, body) = http_post(port, "/api/todo-task-update-sub", &payload);
            assert_eq!(status, 400, "payload={payload}");
            assert!(body.get("error").is_some());
            assert!(body.get("_status").is_none());
        }
    });
}

#[test]
fn post_todo_task_update_sub_unknown_sub_returns_404() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, _, _) = create_todo_master(port, "Update unknown sub", &["A"]);
        let (status, body) = http_post(
            port,
            "/api/todo-task-update-sub",
            &json!({
                "master_task_id": master_id,
                "sub_task_id": "00000000000000000000000000000099",
                "title": "New",
            }),
        );
        assert_eq!(status, 404);
        assert_eq!(body["error"], "Task not found");
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn post_todo_task_add_sub_missing_master_still_400_with_content_present() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-add-sub",
            &json!({ "title": "Sub", "content": "x" }),
        );
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

// --- t3: Sidecar HTTP category surface (list categories, create/list/update category_id) ---

#[test]
fn get_todo_task_list_categories_includes_default_uncategorized() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/todo-task-list-categories");
        assert_eq!(status, 200);
        let cats = body["categories"].as_array().expect("categories array");
        assert!(
            cats.iter().any(|c| {
                c["id"] == crate::services::todo_task::types::DEFAULT_CATEGORY_ID
                    && c["name"] == crate::services::todo_task::types::DEFAULT_CATEGORY_NAME
                    && c["is_default"] == true
            }),
            "must include built-in 待分类, got {cats:?}"
        );
    });
}

#[test]
fn post_todo_task_create_omits_category_id_falls_to_default() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "No cat", "sub_titles": ["S"] }),
        );
        assert_eq!(status, 201);
        assert_eq!(
            body["task"]["category_id"],
            crate::services::todo_task::types::DEFAULT_CATEGORY_ID
        );
    });
}

#[test]
fn post_todo_task_create_with_valid_category_id_assigns_it() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let created_cat = crate::services::todo_task::create_todo_category("Work");
        assert_eq!(created_cat["_status"], 201);
        let cat_id = created_cat["category_id"].as_str().expect("cat id").to_string();

        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({
                "title": "In work",
                "sub_titles": ["S"],
                "category_id": cat_id,
            }),
        );
        assert_eq!(status, 201);
        assert_eq!(body["task"]["category_id"], cat_id);
    });
}

#[test]
fn post_todo_task_create_unknown_category_id_rejects() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/todo-task-create",
            &json!({
                "title": "Bad cat",
                "sub_titles": ["S"],
                "category_id": "cat_missing_zzz",
            }),
        );
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn get_todo_tasks_optional_category_id_filters_and_omission_lists_all() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let created_cat = crate::services::todo_task::create_todo_category("Filter");
        let cat_id = created_cat["category_id"].as_str().expect("id").to_string();

        let (s1, b1) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "In filter", "sub_titles": ["S"], "category_id": cat_id }),
        );
        assert_eq!(s1, 201);
        let in_id = b1["master_task_id"].as_str().unwrap().to_string();

        let (s2, b2) = http_post(
            port,
            "/api/todo-task-create",
            &json!({ "title": "Default one", "sub_titles": ["S"] }),
        );
        assert_eq!(s2, 201);
        let def_id = b2["master_task_id"].as_str().unwrap().to_string();

        let (all_status, all_body) = http_get_with_response(port, "/api/todo-tasks");
        assert_eq!(all_status, 200);
        let all = all_body.as_array().expect("array");
        assert!(all.iter().any(|t| t["master_task_id"] == in_id));
        assert!(all.iter().any(|t| t["master_task_id"] == def_id));

        let (f_status, f_body) =
            http_get_with_response(port, &format!("/api/todo-tasks?category_id={cat_id}"));
        assert_eq!(f_status, 200);
        let filtered = f_body.as_array().expect("filtered array");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["master_task_id"], in_id);
    });
}

#[test]
fn post_todo_task_update_category_id_alone_sets_category() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let created_cat = crate::services::todo_task::create_todo_category("Later");
        let cat_id = created_cat["category_id"].as_str().expect("id").to_string();
        let (master_id, _, _) = create_todo_master(port, "Move me", &["S"]);

        let (status, body) = http_post(
            port,
            "/api/todo-task-update",
            &json!({
                "master_task_id": master_id,
                "category_id": cat_id,
            }),
        );
        assert_eq!(status, 200);
        assert_eq!(body["task"]["category_id"], cat_id);

        let (_, got) = http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(got["category_id"], cat_id);
    });
}

#[test]
fn post_todo_task_update_unknown_category_id_rejects() {
    let fixture = setup_repo_for_todo_task();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (master_id, _, _) = create_todo_master(port, "Keep default", &["S"]);
        let (status, body) = http_post(
            port,
            "/api/todo-task-update",
            &json!({
                "master_task_id": master_id,
                "category_id": "cat_does_not_exist",
            }),
        );
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());

        let (_, got) = http_get(port, &format!("/api/todo-task?id={master_id}"));
        assert_eq!(
            got["category_id"],
            crate::services::todo_task::types::DEFAULT_CATEGORY_ID
        );
    });
}

// --- t1: Host notes selection snapshot Sidecar HTTP ---

const NOTES_SELECTION_API: &str = "/api/notes-selection";

fn empty_notes_selection() -> Value {
    json!({ "date": null, "documents": [] })
}

#[test]
fn get_notes_selection_empty_snapshot_returns_200_not_404() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(status, 200);
        assert_eq!(body, empty_notes_selection());
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn put_notes_selection_full_table_then_get_equals_put() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "2026-06-19",
            "documents": [
                { "id": "11111111111111111111111111111111", "selected": true },
                { "id": "22222222222222222222222222222222", "selected": false }
            ]
        });
        let (put_status, put_body) = http_put(port, NOTES_SELECTION_API, &table);
        assert_eq!(put_status, 200);
        assert_eq!(put_body, table);
        assert!(put_body.get("_status").is_none());

        let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(get_status, 200);
        assert_eq!(get_body, table);
        assert_eq!(get_body, put_body);
    });
}

#[test]
fn put_notes_selection_date_only_all_unselected_get_returns_as_written() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "2026-06-19",
            "documents": [
                { "id": "11111111111111111111111111111111", "selected": false },
                { "id": "22222222222222222222222222222222", "selected": false }
            ]
        });
        let (put_status, put_body) = http_put(port, NOTES_SELECTION_API, &table);
        assert_eq!(put_status, 200);
        assert_eq!(put_body, table);

        let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(get_status, 200);
        assert_eq!(get_body, table);
        let docs = get_body["documents"].as_array().expect("documents");
        assert!(docs.iter().all(|d| d["selected"] == false));
    });
}

#[test]
fn put_notes_selection_empty_snapshot_returns_200_not_404() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let empty = empty_notes_selection();
        let (put_status, put_body) = http_put(port, NOTES_SELECTION_API, &empty);
        assert_eq!(put_status, 200);
        assert_eq!(put_body, empty);

        let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(get_status, 200);
        assert_eq!(get_body, empty);
    });
}

#[test]
fn put_notes_selection_one_selected_true_get_equals_put_without_normalize() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "2026-06-19",
            "documents": [
                { "id": "11111111111111111111111111111111", "selected": false },
                { "id": "22222222222222222222222222222222", "selected": true },
                { "id": "33333333333333333333333333333333", "selected": false }
            ]
        });
        let (put_status, put_body) = http_put(port, NOTES_SELECTION_API, &table);
        assert_eq!(put_status, 200);
        assert_eq!(put_body, table);

        let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(get_status, 200);
        assert_eq!(get_body, table);
        let selected: Vec<_> = get_body["documents"]
            .as_array()
            .expect("documents")
            .iter()
            .filter(|d| d["selected"] == true)
            .collect();
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0]["id"], "22222222222222222222222222222222");
    });
}

#[test]
fn notes_selection_document_ids_match_corpus_files() {
    let catalog = setup_repo_with_catalog();
    let repo_root = catalog.repo_root.clone();
    let id = catalog.id.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "2026-06-19",
            "documents": [{ "id": id, "selected": true }]
        });
        let (put_status, put_body) = http_put(port, NOTES_SELECTION_API, &table);
        assert_eq!(put_status, 200);
        assert_eq!(put_body["documents"][0]["id"], id);

        let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(get_status, 200);
        assert_eq!(get_body["documents"][0]["id"], id);

        let (files_status, files_body) = http_post(
            port,
            "/api/corpus-files",
            &json!({ "ids": [id] }),
        );
        assert_eq!(files_status, 200);
        let items = files_body["items"].as_array().expect("items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["ok"], true);
        assert_eq!(get_body["documents"][0]["id"], id);
    });
}

#[test]
fn notes_selection_only_served_under_existing_sidecar_api() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (api_status, api_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(api_status, 200);
        assert_eq!(api_body, empty_notes_selection());
        assert!(NOTES_SELECTION_API.starts_with("/api/"));

        let (bare_status, bare_body) = http_get(port, "/notes-selection");
        assert_eq!(bare_status, 404);
        assert!(bare_body.get("error").is_some());

        let (put_bare_status, put_bare_body) =
            http_put(port, "/notes-selection", &empty_notes_selection());
        assert!(
            put_bare_status == 404 || put_bare_status == 405,
            "non-/api write must not succeed, got {put_bare_status}: {put_bare_body}"
        );
        assert_ne!(put_bare_status, 200);
    });
}

#[test]
fn notes_selection_write_is_sidecar_http_only_no_mcp_write_tool() {
    for slot in ["todo_task", "cursor_ide", "notes"] {
        if let Some(table) = crate::services::mcp_protocol_adapter::build_slot_tool_table(slot) {
            for tool in &table.tools {
                let name = tool.name.to_lowercase();
                let writes_notes_selection = name.contains("notes_selection")
                    && (name.contains("put")
                        || name.contains("set")
                        || name.contains("write")
                        || name.contains("update"));
                assert!(
                    !writes_notes_selection,
                    "this task must not add an MCP write tool, found {} on {slot}",
                    tool.name
                );
                assert!(
                    !(tool.api_path.contains("notes-selection")
                        && !matches!(
                            tool.method,
                            crate::services::mcp_protocol_adapter::HttpMethod::Get
                        )),
                    "notes selection write must stay Sidecar HTTP, found {} {} on {slot}",
                    tool.name,
                    tool.api_path
                );
            }
        }
    }

    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "2026-06-19",
            "documents": [{ "id": "11111111111111111111111111111111", "selected": true }]
        });
        let (put_status, put_body) = http_put(port, NOTES_SELECTION_API, &table);
        assert_eq!(put_status, 200);
        assert_eq!(put_body, table);

        let (post_status, post_body) = http_post(port, NOTES_SELECTION_API, &table);
        assert_eq!(
            post_status, 405,
            "write is PUT on Sidecar HTTP, not POST: {post_body}"
        );
    });
}

// --- t5: Notes UI writes selection snapshot via existing Sidecar HTTP ---
//
// Proof surface is Host GET=PUT (t1). Do not add a new frontend test harness.
// UI wiring is checked by reading the Notes consumer source (same pattern as t4).

fn t5_repo_file(rel: &str) -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.push(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn t5_function_slice<'a>(src: &'a str, marker: &str) -> &'a str {
    let start = src.find(marker).unwrap_or(src.len());
    let rest = &src[start..];
    let end = rest.len().min(2400);
    &rest[..end]
}

fn t5_assert_get_equals_put(port: u16, table: &Value) {
    let (put_status, put_body) = http_put(port, NOTES_SELECTION_API, table);
    assert_eq!(put_status, 200);
    assert_eq!(&put_body, table);
    let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
    assert_eq!(get_status, 200);
    assert_eq!(&get_body, table);
    assert_eq!(get_body, put_body);
}

/// Normal: enter-Notes / date-change UI write is a full-table PUT; GET equals that PUT.
#[test]
fn t5_ui_enter_or_date_change_put_then_get_equals_put() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "20260817",
            "documents": [
                { "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "selected": false },
                { "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "selected": false }
            ]
        });
        t5_assert_get_equals_put(port, &table);
    });
}

/// Normal: document-open UI write is a full-table PUT with at most one selected=true.
#[test]
fn t5_ui_document_change_put_then_get_equals_put() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "20260817",
            "documents": [
                { "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "selected": false },
                { "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "selected": true }
            ]
        });
        t5_assert_get_equals_put(port, &table);
        let selected: Vec<_> = table["documents"]
            .as_array()
            .expect("documents")
            .iter()
            .filter(|d| d["selected"] == true)
            .collect();
        assert_eq!(selected.len(), 1);
    });
}

/// Boundary: date-only write may list documents, all selected=false; GET equals PUT.
#[test]
fn t5_ui_date_only_put_all_unselected_get_equals_put() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "20260817",
            "documents": [
                { "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "selected": false },
                { "id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "selected": false }
            ]
        });
        t5_assert_get_equals_put(port, &table);
        let docs = table["documents"].as_array().expect("documents");
        assert!(docs.iter().all(|d| d["selected"] == false));
    });
}

/// Boundary: leave Notes business writes empty snapshot; GET equals that PUT.
#[test]
fn t5_ui_leave_notes_put_empty_then_get_equals_put() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let prior = json!({
            "date": "20260817",
            "documents": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "selected": true }]
        });
        t5_assert_get_equals_put(port, &prior);
        t5_assert_get_equals_put(port, &empty_notes_selection());
    });
}

/// Boundary: Hub / shell-close is not a leave — last PUT remains; GET still equals it.
#[test]
fn t5_hub_or_shell_close_does_not_put_empty_get_keeps_last_put() {
    let fixture = setup_repo_with_corpus();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let table = json!({
            "date": "20260817",
            "documents": [{ "id": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "selected": true }]
        });
        t5_assert_get_equals_put(port, &table);
        let (get_status, get_body) = http_get(port, NOTES_SELECTION_API);
        assert_eq!(get_status, 200);
        assert_eq!(get_body, table);
        assert_ne!(get_body, empty_notes_selection());
    });
}

/// Normal: apiClient exposes writeNotesSelectionSnapshot over existing Sidecar PUT.
#[test]
fn t5_api_client_writes_snapshot_via_sidecar_http_put() {
    let api = t5_repo_file("frontend/js/apiClient.js");
    assert!(
        api.contains("writeNotesSelectionSnapshot"),
        "apiClient.js must export writeNotesSelectionSnapshot"
    );
    let write = t5_function_slice(&api, "function writeNotesSelectionSnapshot");
    assert!(
        write.contains("/api/notes-selection"),
        "write must use t1 Sidecar path /api/notes-selection"
    );
    assert!(
        write.contains("PUT") || write.contains("putJson") || write.contains("method: 'PUT'"),
        "write must be HTTP PUT on Sidecar, not POST/invoke"
    );
    assert!(
        !write.contains("invoke("),
        "write must stay on Sidecar HTTP (apiClient), not Tauri/MCP invoke"
    );
}

/// Boundary: clearNotesSelectionSnapshot writes the empty snapshot.
#[test]
fn t5_api_client_clears_snapshot_with_empty_table() {
    let api = t5_repo_file("frontend/js/apiClient.js");
    assert!(
        api.contains("clearNotesSelectionSnapshot"),
        "apiClient.js must export clearNotesSelectionSnapshot"
    );
    let clear = t5_function_slice(&api, "function clearNotesSelectionSnapshot");
    assert!(
        clear.contains("null") && clear.contains("documents"),
        "clear must write {{ date: null, documents: [] }}"
    );
}

/// Normal: enter Notes and sidebar selectDate overwrite the full-day group.
#[test]
fn t5_enter_notes_and_select_date_write_full_day_snapshot() {
    let main = t5_repo_file("frontend/js/main.js");
    let sidebar = t5_repo_file("frontend/js/components/sidebar.js");
    let mount_wb = t5_function_slice(&main, "function mountWorkbench");
    assert!(
        mount_wb.contains("writeNotesSelectionSnapshot"),
        "entering Notes (mountWorkbench) must write the current selection snapshot"
    );
    let select = t5_function_slice(&sidebar, "export function selectDate");
    assert!(
        select.contains("writeNotesSelectionSnapshot"),
        "sidebar selectDate must full-table overwrite the day's group"
    );
}

/// Normal: document open in main / cards overwrites with at most one selected=true.
#[test]
fn t5_document_open_in_main_and_cards_writes_snapshot() {
    let main = t5_repo_file("frontend/js/main.js");
    let cards = t5_repo_file("frontend/js/components/cards.js");
    assert!(
        main.contains("writeNotesSelectionSnapshot"),
        "main.js document-open path must write the selection snapshot"
    );
    assert!(
        cards.contains("writeNotesSelectionSnapshot"),
        "cards.js document-open path must write the selection snapshot"
    );
}

/// Boundary: only leaving Notes business clears; Hub / shell-close must not.
#[test]
fn t5_clear_only_when_leaving_notes_not_hub_or_shell_close() {
    let main = t5_repo_file("frontend/js/main.js");
    let shell = t5_repo_file("frontend/js/home-entry-shell/shell.js");
    let note_assistant = t5_repo_file("frontend/js/note-assistant.js");
    let mount_plan = t5_function_slice(&main, "function mountPlanTasksRoute");
    assert!(
        mount_plan.contains("clearNotesSelectionSnapshot"),
        "leaving Notes for Todos (mountPlanTasksRoute) must write the empty snapshot"
    );
    assert!(
        !shell.contains("clearNotesSelectionSnapshot") && !shell.contains("writeNotesSelectionSnapshot"),
        "Hub small window must not clear or rewrite the selection snapshot"
    );
    assert!(
        !note_assistant.contains("clearNotesSelectionSnapshot")
            && !note_assistant.contains("writeNotesSelectionSnapshot"),
        "close-shell / note-assistant must not clear the selection snapshot"
    );
}

/// Exception: write stays Sidecar HTTP; no MCP write tool and no Tauri write map.
#[test]
fn t5_write_is_sidecar_http_only_no_mcp_or_tauri_write_map() {
    let write_map = t5_repo_file("frontend/js/writeApiInvokeMap.js");
    assert!(
        !write_map.contains("notes-selection") && !write_map.contains("notes_selection"),
        "must not add a Tauri write invoke for notes-selection"
    );
    for slot in ["todo_task", "cursor_ide", "notes"] {
        if let Some(table) = crate::services::mcp_protocol_adapter::build_slot_tool_table(slot) {
            for tool in &table.tools {
                let name = tool.name.to_lowercase();
                let writes = name.contains("notes_selection")
                    && (name.contains("put")
                        || name.contains("set")
                        || name.contains("write")
                        || name.contains("update"));
                assert!(!writes, "must not add MCP write tool {}, slot {slot}", tool.name);
            }
        }
    }
}

/// Exception: do not add a new frontend test harness; GET=PUT remains the proof.
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
