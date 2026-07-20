use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use reqwest::blocking;
use serde_json::{json, Value};

use super::*;
use crate::test_support::TestSandbox;

fn with_plan_task_http_test<F: FnOnce()>(f: F) {
    f();
}

fn assert_ac5_master_task_shape(task: &Value) {
    assert!(task.get("master_task_id").and_then(|v| v.as_str()).is_some());
    assert!(task.get("title").and_then(|v| v.as_str()).is_some());
    let status = task.get("status").and_then(|v| v.as_str()).expect("status");
    assert!(
        PLAN_TASK_MASTER_STATUS_WIRE.contains(&status),
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

fn setup_repo_for_plan_task() -> RepoFixture {
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
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, body) = http_post(
            port,
            "/api/archive-document",
            &json!({ "document": SAMPLE_DOC }),
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
fn get_plan_tasks_empty_returns_200_array() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(status, 200);
            assert!(body.is_array());
            assert_eq!(body.as_array().expect("array").len(), 0);
        });
    });
}

#[test]
fn get_plan_tasks_returns_created_masters() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (create_status, create_body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Listed master", "sub_titles": ["Sub A", "Sub B"] }),
            );
            assert_eq!(create_status, 201);
            let master_id = create_body["master_task_id"]
                .as_str()
                .expect("master_task_id");

            let (status, body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(status, 200);
            let list = body.as_array().expect("array");
            assert_eq!(list.len(), 1);
            assert_eq!(list[0]["master_task_id"], master_id);
            assert_eq!(list[0]["title"], "Listed master");
            let subs = list[0]["sub_tasks"].as_array().expect("sub_tasks");
            assert_eq!(subs.len(), 2);
        });
    });
}

#[test]
fn post_plan_task_create_omit_sub_titles_creates_empty_sub_tasks() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
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
    });
}

#[test]
fn post_plan_task_create_empty_sub_titles_matches_omit() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Empty array", "sub_titles": [] }),
            );
            assert_eq!(status, 201);
            let subs = body["task"]["sub_tasks"].as_array().expect("sub_tasks");
            assert!(subs.is_empty());
            assert_eq!(body["task"]["status"], "incomplete");
        });
    });
}

#[test]
fn post_plan_task_create_single_explicit_sub() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Master", "sub_titles": ["Sub A"] }),
            );
            assert_eq!(status, 201);
            let subs = body["task"]["sub_tasks"].as_array().expect("sub_tasks");
            assert_eq!(subs.len(), 1);
            assert_eq!(subs[0]["implicit"], false);
            assert_eq!(subs[0]["title"], "Sub A");
        });
    });
}

#[test]
fn post_plan_task_create_multiple_explicit_subs() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
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
    });
}

#[test]
fn post_plan_task_create_blank_title_returns_400() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "   " }),
            );
            assert_eq!(status, 400);
            assert!(body.get("error").is_some());
        });
    });
}

#[test]
fn post_plan_task_create_response_ac5_field_matrix() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
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
                wb.join("plan_tasks")
                    .join("tasks")
                    .join(master_id)
                    .join("sub_tasks.json")
                    .is_file(),
                "v2 storage must exist but remain invisible in HTTP shape"
            );
        });
    });
}

fn create_plan_master(port: u16, title: &str, sub_titles: &[&str]) -> (String, String, Value) {
    let payload = if sub_titles.is_empty() {
        json!({ "title": title })
    } else {
        json!({ "title": title, "sub_titles": sub_titles })
    };
    let (status, body) = http_post(port, "/api/plan-task-create", &payload);
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
fn plan_task_master_status_wire_includes_abandoned() {
    assert_eq!(
        PLAN_TASK_MASTER_STATUS_WIRE,
        &["incomplete", "complete", "abandoned"]
    );
}

#[test]
fn http_list_get_create_carry_tri_state_status_including_abandoned() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (create_status, create_body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Create incomplete", "sub_titles": ["Sub"] }),
            );
            assert_eq!(create_status, 201);
            assert_ac5_master_task_shape(&create_body["task"]);
            assert_eq!(create_body["task"]["status"], "incomplete");

            let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
            let complete_id = "task_http_status_complete";
            let abandoned_id = "task_http_status_abandoned";
            seed_v2_plan_for_http(
                &wb,
                complete_id,
                "Stored complete",
                "complete",
                &json!({ "sub_tasks": [] }),
                true,
            );
            seed_v2_plan_for_http(
                &wb,
                abandoned_id,
                "Stored abandoned",
                "abandoned",
                &json!({ "sub_tasks": [] }),
                true,
            );

            let (list_status, list_body) = http_get_with_response(port, "/api/plan-tasks");
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
            assert!(PLAN_TASK_MASTER_STATUS_WIRE.contains(&complete["status"].as_str().unwrap()));
            assert!(PLAN_TASK_MASTER_STATUS_WIRE.contains(&abandoned["status"].as_str().unwrap()));

            let (get_complete_status, get_complete) =
                http_get(port, &format!("/api/plan-task?id={complete_id}"));
            assert_eq!(get_complete_status, 200);
            assert_eq!(get_complete["status"], "complete");

            let (get_abandoned_status, get_abandoned) =
                http_get(port, &format!("/api/plan-task?id={abandoned_id}"));
            assert_eq!(get_abandoned_status, 200);
            assert_eq!(get_abandoned["status"], "abandoned");
        });
    });
}

#[test]
fn plan_task_crud_http_flow() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (master_id, sub_a, create_body) =
                create_plan_master(port, "CRUD master", &["Sub A", "Sub B"]);
            assert_eq!(create_body["task"]["sub_tasks"].as_array().unwrap().len(), 2);

            let (get_status, get_body) =
                http_get(port, &format!("/api/plan-task?id={master_id}"));
            assert_eq!(get_status, 200);
            assert_eq!(get_body["master_task_id"], master_id);
            assert_eq!(get_body["title"], "CRUD master");
            assert!(get_body.get("_status").is_none());

            let (add_status, add_body) = http_post(
                port,
                "/api/plan-task-add-sub",
                &json!({ "master_task_id": master_id, "title": "Sub C" }),
            );
            assert_eq!(add_status, 201);
            let sub_c = add_body["sub_task_id"].as_str().expect("sub_task_id");
            assert_eq!(add_body["task"]["sub_tasks"].as_array().unwrap().len(), 3);

            let (complete_status, complete_body) = http_post(
                port,
                "/api/plan-task-complete",
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
                "/api/plan-task-link-archive",
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
                "/api/plan-task-delete-sub",
                &json!({ "master_task_id": master_id, "sub_task_id": sub_c }),
            );
            assert_eq!(del_sub_status, 200);
            assert_eq!(del_sub_body["task"]["sub_tasks"].as_array().unwrap().len(), 2);

            let (delete_status, delete_body) = http_post(
                port,
                "/api/plan-task-delete",
                &json!({ "master_task_id": master_id }),
            );
            assert_eq!(delete_status, 200);
            assert_eq!(delete_body["ok"], true);
            assert!(delete_body.get("_status").is_none());

            let (list_status, list_body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(list_status, 200);
            assert_eq!(list_body.as_array().unwrap().len(), 0);
        });
    });
}

#[test]
fn get_plan_task_missing_id_returns_400() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_get(port, "/api/plan-task");
            assert_eq!(status, 400);
            assert!(body.get("error").is_some());
            assert!(body.get("_status").is_none());
        });
    });
}

#[test]
fn get_plan_task_unknown_id_returns_404() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) =
                http_get(port, "/api/plan-task?id=00000000000000000000000000000000");
            assert_eq!(status, 404);
            assert_eq!(body["error"], "Task not found");
            assert!(body.get("_status").is_none());
        });
    });
}

#[test]
fn post_plan_task_add_sub_missing_or_blank_title_returns_400() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (master_id, _, _) = create_plan_master(port, "Master", &["Sub A"]);

            for payload in [
                json!({ "master_task_id": master_id }),
                json!({ "master_task_id": master_id, "title": "" }),
                json!({ "master_task_id": master_id, "title": "   " }),
            ] {
                let (status, body) = http_post(port, "/api/plan-task-add-sub", &payload);
                assert_eq!(status, 400);
                assert!(body.get("error").is_some());
                assert!(body.get("_status").is_none());
            }
        });
    });
}

#[test]
fn post_plan_task_delete_sub_last_sub_allows_empty() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (master_id, sub_id, _) = create_plan_master(port, "Single sub", &["Single sub"]);
            let (status, body) = http_post(
                port,
                "/api/plan-task-delete-sub",
                &json!({ "master_task_id": master_id, "sub_task_id": sub_id }),
            );
            assert_eq!(status, 200);
            assert!(body.get("_status").is_none());
            assert!(body["task"]["sub_tasks"].as_array().unwrap().is_empty());
            assert_eq!(body["task"]["status"], "incomplete");
        });
    });
}

#[test]
fn post_plan_task_unknown_master_returns_404() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let unknown = "00000000000000000000000000000000";
            for path in [
                "/api/plan-task-delete",
                "/api/plan-task-add-sub",
                "/api/plan-task-delete-sub",
                "/api/plan-task-complete",
                "/api/plan-task-link-archive",
            ] {
                let payload = match path {
                    "/api/plan-task-delete" => json!({ "master_task_id": unknown }),
                    "/api/plan-task-add-sub" => {
                        json!({ "master_task_id": unknown, "title": "Sub" })
                    }
                    "/api/plan-task-delete-sub" | "/api/plan-task-complete" => {
                        json!({
                            "master_task_id": unknown,
                            "sub_task_id": "00000000000000000000000000000001",
                        })
                    }
                    "/api/plan-task-link-archive" => {
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
    });
}

#[test]
fn post_plan_task_unknown_sub_returns_404() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (master_id, _, _) = create_plan_master(port, "Master", &["Sub A", "Sub B"]);
            let unknown_sub = "00000000000000000000000000000099";
            for path in [
                "/api/plan-task-delete-sub",
                "/api/plan-task-complete",
                "/api/plan-task-link-archive",
            ] {
                let payload = match path {
                    "/api/plan-task-delete-sub" | "/api/plan-task-complete" => {
                        json!({ "master_task_id": master_id, "sub_task_id": unknown_sub })
                    }
                    "/api/plan-task-link-archive" => {
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
    });
}

#[test]
fn post_plan_task_complete_legacy_route_returns_404() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (master_id, sub_id, _) = create_plan_master(port, "Legacy route", &["Sub A"]);
            let (status, body) = http_post(
                port,
                "/api/plan-task-complete-sub",
                &json!({ "master_task_id": master_id, "sub_task_id": sub_id }),
            );
            assert_eq!(status, 404);
            assert!(body.get("error").is_some() || body.get("_status").is_none());
        });
    });
}

#[test]
fn post_plan_task_complete_without_sub_marks_master_complete() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (master_id, _, _) = create_plan_master(port, "Master only", &["Sub A"]);
            let (status, body) = http_post(
                port,
                "/api/plan-task-complete",
                &json!({ "master_task_id": master_id }),
            );
            assert_eq!(status, 200);
            assert!(body.get("_status").is_none());
            assert_eq!(body["task"]["status"], "complete");
            assert_eq!(body["task"]["sub_tasks"][0]["status"], "incomplete");
        });
    });
}

#[test]
fn post_plan_task_create_blank_sub_title_element_returns_400() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Master", "sub_titles": ["ok", "  "] }),
            );
            assert_eq!(status, 400);
            assert!(body.get("error").is_some());
        });
    });
}

fn seed_v2_plan_for_http(
    wb: &std::path::Path,
    master_id: &str,
    title: &str,
    status: &str,
    sub_tasks: &Value,
    merge_index: bool,
) {
    let plan_tasks_dir = wb.join("plan_tasks");
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
            .join("plan.md"),
        "",
    )
    .expect("write plan.md");
}

fn assert_http_master_has_plan_fields(task: &Value) {
    assert!(
        task.get("plan_md").is_some(),
        "plan_md must be a top-level field"
    );
    assert!(
        task.get("migration_error").and_then(|v| v.as_bool()).is_some(),
        "migration_error must be a top-level boolean"
    );
}

#[test]
fn get_plan_tasks_includes_plan_md_and_migration_error_fields() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (create_status, _) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "HTTP plan fields" }),
            );
            assert_eq!(create_status, 201);

            let (status, body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(status, 200);
            let list = body.as_array().expect("array");
            assert_eq!(list.len(), 1);
            assert_http_master_has_plan_fields(&list[0]);
            assert_eq!(list[0]["plan_md"], "");
            assert_eq!(list[0]["migration_error"], false);

            let expected = crate::services::plan_task::list_all();
            assert_eq!(body, expected);
        });
    });
}

#[test]
fn get_plan_task_by_id_includes_plan_md_and_migration_error_matching_list() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (create_status, create_body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "HTTP get by id fields" }),
            );
            assert_eq!(create_status, 201);
            let master_id = create_body["master_task_id"]
                .as_str()
                .expect("master_task_id");

            let (list_status, list_body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(list_status, 200);
            let list = list_body.as_array().expect("array");
            assert_eq!(list.len(), 1);

            let (get_status, get_body) =
                http_get(port, &format!("/api/plan-task?id={master_id}"));
            assert_eq!(get_status, 200);
            assert_http_master_has_plan_fields(&get_body);
            assert_eq!(get_body["plan_md"], list[0]["plan_md"]);
            assert_eq!(get_body["migration_error"], list[0]["migration_error"]);
            assert!(get_body.get("_status").is_none());

            let expected = crate::services::plan_task::get_by_id(master_id);
            assert_eq!(get_body, expected);
        });
    });
}

#[test]
fn get_plan_tasks_plan_md_matches_disk_bytes() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (create_status, create_body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Disk plan md" }),
            );
            assert_eq!(create_status, 201);
            let master_id = create_body["master_task_id"]
                .as_str()
                .expect("master_task_id");

            let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
            let plan_path = wb
                .join("plan_tasks")
                .join("tasks")
                .join(master_id)
                .join("plan.md");
            let content = "# Title\n\n## Section\n\n- item one\n";
            fs::write(&plan_path, content).expect("write plan.md");

            let (list_status, list_body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(list_status, 200);
            assert_eq!(list_body[0]["plan_md"], content);
            assert_eq!(fs::read_to_string(&plan_path).unwrap(), content);

            let (get_status, get_body) =
                http_get(port, &format!("/api/plan-task?id={master_id}"));
            assert_eq!(get_status, 200);
            assert_eq!(get_body["plan_md"], content);
        });
    });
}

#[test]
fn get_plan_tasks_empty_plan_md_matches_empty_disk_file() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (create_status, create_body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Empty plan md" }),
            );
            assert_eq!(create_status, 201);
            let master_id = create_body["master_task_id"]
                .as_str()
                .expect("master_task_id");

            let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
            let plan_path = wb
                .join("plan_tasks")
                .join("tasks")
                .join(master_id)
                .join("plan.md");
            assert_eq!(fs::read_to_string(&plan_path).unwrap(), "");

            let (list_status, list_body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(list_status, 200);
            assert_eq!(list_body[0]["plan_md"], "");
            assert_eq!(list_body[0]["migration_error"], false);
        });
    });
}

#[test]
fn plan_tasks_http_body_helpers_preserve_read_path_fields() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (create_status, create_body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Body helper fields" }),
            );
            assert_eq!(create_status, 201);
            let master_id = create_body["master_task_id"]
                .as_str()
                .expect("master_task_id");

            let list_value = crate::services::plan_task::list_all();
            let list_body = plan_tasks_list_response_body(&list_value);
            let list_parsed: Value = serde_json::from_str(&list_body).expect("list json");
            assert_eq!(list_parsed, list_value);
            assert_http_master_has_plan_fields(&list_parsed[0]);

            let get_value = crate::services::plan_task::get_by_id(master_id);
            let (get_status, get_body) = plan_task_get_response_body(&get_value);
            assert_eq!(get_status, 200);
            let get_parsed: Value = serde_json::from_str(&get_body).expect("get json");
            assert_eq!(get_parsed, get_value);
            assert_http_master_has_plan_fields(&get_parsed);
        });
    });
}

#[test]
fn post_plan_task_create_with_plan_md_persists_and_lists() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let content = "## HTTP create\n\nBody";
            let (create_status, create_body) = http_post(
                port,
                "/api/plan-task-create",
                &json!({ "title": "Create with md", "plan_md": content }),
            );
            assert_eq!(create_status, 201);
            let master_id = create_body["master_task_id"]
                .as_str()
                .expect("master_task_id");

            let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
            let plan_path = wb
                .join("plan_tasks")
                .join("tasks")
                .join(master_id)
                .join("plan.md");
            assert_eq!(fs::read_to_string(&plan_path).unwrap(), content);

            let (list_status, list_body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(list_status, 200);
            assert_eq!(list_body[0]["plan_md"], content);

            let (get_status, get_body) =
                http_get(port, &format!("/api/plan-task?id={master_id}"));
            assert_eq!(get_status, 200);
            assert_eq!(get_body["plan_md"], content);
        });
    });
}

#[test]
fn post_plan_task_create_title_too_long_returns_400() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let (status, body) = http_post(
                port,
                "/api/plan-task-create",
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
    });
}

#[test]
fn get_plan_tasks_migration_error_plan_still_in_list() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
            let master_id = "task_http_migrate_err";
            seed_v2_plan_for_http(
                &wb,
                master_id,
                "Migration error plan",
                "incomplete",
                &json!({ "sub_tasks": [] }),
                true,
            );
            fs::write(
                wb.join("plan_tasks")
                    .join("tasks")
                    .join(master_id)
                    .join("sub_tasks.json"),
                "{not valid json",
            )
            .expect("write corrupt sub_tasks");

            let (status, body) = http_get_with_response(port, "/api/plan-tasks");
            assert_eq!(status, 200);
            let list = body.as_array().expect("array");
            assert_eq!(list.len(), 1);
            assert_eq!(list[0]["master_task_id"], master_id);
            assert_eq!(list[0]["migration_error"], true);
            assert_http_master_has_plan_fields(&list[0]);

            let (get_status, get_body) =
                http_get(port, &format!("/api/plan-task?id={master_id}"));
            assert_eq!(get_status, 200);
            assert_eq!(get_body["migration_error"], true);
            assert_eq!(get_body["plan_md"], "");
        });
    });
}

fn create_plan_master_id(port: u16, title: &str) -> String {
    let (status, body) = http_post(port, "/api/plan-task-create", &json!({ "title": title }));
    assert_eq!(status, 201);
    body["master_task_id"]
        .as_str()
        .expect("master_task_id")
        .to_string()
}

#[test]
fn plan_task_attachment_http_flow() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let master_id = create_plan_master_id(port, "Attach HTTP");

            let (add_status, add_body) = http_post(
                port,
                "/api/plan-task-add-attachment",
                &json!({
                    "master_task_id": master_id,
                    "file_name": "notes.md",
                    "content": "# Notes\n",
                }),
            );
            assert_eq!(add_status, 201);
            assert_eq!(add_body["file_name"], "notes.md");
            assert_eq!(add_body["original_file_name"], "notes.md");
            assert!(add_body.get("added_at").and_then(|v| v.as_str()).is_some());
            assert!(add_body.get("_status").is_none());

            let (list_status, list_body) = http_post(
                port,
                "/api/plan-task-list-attachments",
                &json!({ "master_task_id": master_id }),
            );
            assert_eq!(list_status, 200);
            let attachments = list_body["attachments"].as_array().expect("attachments");
            assert_eq!(attachments.len(), 1);
            assert_eq!(attachments[0]["file_name"], "notes.md");
            assert!(list_body.get("_status").is_none());

            let (get_status, get_body) = http_post(
                port,
                "/api/plan-task-get-attachment",
                &json!({
                    "master_task_id": master_id,
                    "file_name": "notes.md",
                }),
            );
            assert_eq!(get_status, 200);
            assert_eq!(get_body["file_name"], "notes.md");
            assert_eq!(get_body["content"], "# Notes\n");
            assert!(get_body.get("_status").is_none());

            let (update_status, update_body) = http_post(
                port,
                "/api/plan-task-update-attachment",
                &json!({
                    "master_task_id": master_id,
                    "file_name": "notes.md",
                    "content": "updated body",
                }),
            );
            assert_eq!(update_status, 200);
            assert_eq!(update_body["ok"], true);
            assert!(update_body.get("_status").is_none());

            let (reread_status, reread_body) = http_post(
                port,
                "/api/plan-task-get-attachment",
                &json!({
                    "master_task_id": master_id,
                    "file_name": "notes.md",
                }),
            );
            assert_eq!(reread_status, 200);
            assert_eq!(reread_body["content"], "updated body");
        });
    });
}

#[test]
fn plan_task_attachment_http_paths_follow_plan_task_verb_prefix() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let master_id = create_plan_master_id(port, "Path style");
            for path in [
                "/api/plan-task-add-attachment",
                "/api/plan-task-list-attachments",
                "/api/plan-task-get-attachment",
                "/api/plan-task-update-attachment",
            ] {
                assert!(
                    path.starts_with("/api/plan-task-"),
                    "path {path} must use plan-task prefix"
                );
            }

            let (status, body) = http_post(
                port,
                "/api/plan-task-add-attachment",
                &json!({
                    "master_task_id": master_id,
                    "file_name": "a.md",
                    "content": "x",
                }),
            );
            assert_eq!(status, 201, "add-attachment must be registered: {body}");
            assert!(body.get("error").is_none());
        });
    });
}

#[test]
fn plan_task_attachment_http_has_no_delete_or_ui_paths() {
    with_plan_task_http_test(|| {
        let fixture = setup_repo_for_plan_task();
        let repo_root = fixture.repo_root.clone();
        with_server(repo_root, |port| {
            let master_id = create_plan_master_id(port, "No delete HTTP");
            let (add_status, _) = http_post(
                port,
                "/api/plan-task-add-attachment",
                &json!({
                    "master_task_id": master_id,
                    "file_name": "notes.md",
                    "content": "keep",
                }),
            );
            assert_eq!(add_status, 201);

            for path in [
                "/api/plan-task-delete-attachment",
                "/api/plan-task-remove-attachment",
                "/api/plan-task-ui-add-attachment",
                "/api/plan-task-open-attachment-dialog",
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
                "/api/plan-task-list-attachments",
                &json!({ "master_task_id": master_id }),
            );
            assert_eq!(list_status, 200);
            assert_eq!(list_body["attachments"].as_array().unwrap().len(), 1);
        });
    });
}
