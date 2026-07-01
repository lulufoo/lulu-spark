use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use reqwest::blocking;
use serde_json::{json, Value};

use super::*;

fn ephemeral_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port()
}

fn setup_repo_without_index() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir corpus");
    crate::config::settings::write_test_config(dir.path(), &corpus, None);
    let repo_root = dir.path().to_path_buf();
    (dir, repo_root)
}

fn setup_repo_with_corpus() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(corpus.join("digest")).expect("mkdir digest");
    fs::write(corpus.join("index.json"), br#"{"entries":[]}"#).expect("index");
    fs::write(corpus.join("digest/note.md"), b"digest body").expect("digest file");
    crate::config::settings::write_test_config(dir.path(), &corpus, None);
    let repo_root = dir.path().to_path_buf();
    (dir, repo_root)
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

fn setup_repo_for_read_later() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir corpus");
    let cache = dir.path().join("cache");
    fs::create_dir_all(&cache).expect("mkdir cache");
    fs::write(
        dir.path().join("config.toml"),
        format!(
            r#"workbench_knowledge_root = "{}"
knowledge_corpus_root = "{}"
cache_dir = "{}"
"#,
            dir.path().display(),
            corpus.display(),
            cache.display()
        ),
    )
    .expect("write config");
    crate::config::settings::set_test_config_dir(Some(dir.path().to_path_buf()));
    let repo_root = dir.path().to_path_buf();
    (dir, repo_root)
}

fn setup_repo_with_catalog() -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().expect("tmpdir");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(corpus.join("digest/ai")).expect("mkdir digest");
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
    fs::write(corpus.join("index.json"), index.to_string()).expect("index");
    fs::write(corpus.join("digest/ai/note.md"), b"digest body").expect("digest file");
    crate::config::settings::write_test_config(dir.path(), &corpus, None);
    let repo_root = dir.path().to_path_buf();
    (dir, repo_root, id.to_string())
}

fn with_server<F: FnOnce(u16)>(repo_root: PathBuf, f: F) {
    let port = ephemeral_port();
    let handle = start(repo_root, port).expect("start server");
    thread::sleep(Duration::from_millis(50));
    f(port);
    stop(handle);
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn get_corpus_catalog_latest_per_topic() {
    let (_dir, repo_root, id) = setup_repo_with_catalog();
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
    let (_dir, repo_root, _) = setup_repo_with_catalog();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-catalog?mode=unknown");
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn post_corpus_files_returns_batch() {
    let (_dir, repo_root, id) = setup_repo_with_catalog();
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
    let (_dir, repo_root, _) = setup_repo_with_catalog();
    with_server(repo_root, |port| {
        let (status, body) = http_post(port, "/api/corpus-files", &json!({ "ids": [] }));
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn get_corpus_index_matches_workbench_read() {
    let (_dir, repo_root) = setup_repo_with_corpus();
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
    let (_dir, repo_root) = setup_repo_with_corpus();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-file?layer=digest&path=note.md");
        assert_eq!(status, 200);
        assert_eq!(body["content"], "digest body");
    });
}

#[test]
fn get_status_returns_ok_and_http_port() {
    let (_dir, repo_root) = setup_repo_with_corpus();
    let port = ephemeral_port();
    let handle = start(repo_root, port).expect("start");
    thread::sleep(Duration::from_millis(50));
    let (status, body) = http_get(port, "/api/status");
    stop(handle);
    crate::config::settings::set_test_config_dir(None);
    assert_eq!(status, 200);
    assert_eq!(body["ok"], true);
    assert_eq!(body["http_port"], port);
}

#[test]
fn get_corpus_file_raw_layer_returns_400() {
    let (_dir, repo_root) = setup_repo_with_corpus();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-file?layer=raw&path=x");
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn get_corpus_asset_raw_returns_base64_png() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(corpus.join("raw/ai")).expect("mkdir raw");
    fs::write(corpus.join("raw/ai/note.png"), b"\x89PNG\r\n").expect("png");
    fs::write(corpus.join("raw/ai/note.md"), b"# note").expect("md");
    crate::config::settings::write_test_config(dir.path(), &corpus, None);
    let repo_root = dir.path().to_path_buf();
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
    let (_dir, repo_root) = setup_repo_without_index();
    with_server(repo_root, |port| {
        let (status, body) = http_get(port, "/api/corpus-index");
        assert_eq!(status, 404);
        assert!(body.get("error").is_some());
        assert!(body.get("_status").is_none());
    });
}

#[test]
fn start_fails_when_port_in_use_without_panic() {
    let (_dir, repo_root) = setup_repo_with_corpus();
    let port = ephemeral_port();
    let _guard = TcpListener::bind(format!("127.0.0.1:{port}")).expect("occupy port");
    let result = start(repo_root, port);
    crate::config::settings::set_test_config_dir(None);
    assert!(result.is_err());
}

fn setup_repo_for_archive() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tmpdir");
    let repo_root = dir.path().to_path_buf();
    let corpus = repo_root.join("corpus");
    fs::create_dir_all(corpus.join("raw")).expect("mkdir raw");
    fs::create_dir_all(corpus.join("digest")).expect("mkdir digest");
    fs::write(corpus.join("index.json"), br#"{"entries":{}}"#).expect("index");
    crate::config::settings::write_test_config(&repo_root, &corpus, None);
    (dir, repo_root)
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
    let (_dir, repo_root) = setup_repo_for_archive();
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
    let (_dir, repo_root) = setup_repo_with_corpus();
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
    let (_dir, repo_root) = setup_repo_with_corpus();
    let state = LocalHttpState::new();
    let port = ephemeral_port();
    state.try_start(repo_root, port);
    assert!(state.is_ready());
    thread::sleep(Duration::from_millis(50));
    let (status, _) = http_get(port, "/api/corpus-index");
    assert_eq!(status, 200);
    state.stop();
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn local_http_state_stop_clears_http_ready_and_releases_port() {
    let (_dir, repo_root) = setup_repo_with_corpus();
    let state = LocalHttpState::new();
    let port = ephemeral_port();
    state.try_start(repo_root, port);
    assert!(state.is_ready());
    thread::sleep(Duration::from_millis(50));
    state.stop();
    assert!(!state.is_ready());
    assert!(!crate::wait_for_port(port, Duration::from_millis(200)));
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn local_http_state_three_cycles_no_port_leak() {
    let (_dir, repo_root) = setup_repo_with_corpus();
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
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn local_http_state_bind_failure_keeps_http_ready_false() {
    let (_dir, repo_root) = setup_repo_with_corpus();
    let port = ephemeral_port();
    let _guard = TcpListener::bind(format!("127.0.0.1:{port}")).expect("occupy port");
    let state = LocalHttpState::new();
    state.try_start(repo_root, port);
    assert!(!state.is_ready());
    crate::config::settings::set_test_config_dir(None);
}

#[test]
fn get_read_later_empty_returns_200_array() {
    let (_dir, repo_root) = setup_repo_for_read_later();
    with_server(repo_root, |port| {
        let (status, body) = http_get_with_response(port, "/api/read-later");
        assert_eq!(status, 200);
        assert!(body.is_array());
        assert_eq!(body.as_array().expect("array").len(), 0);
    });
}

#[test]
fn post_read_later_creates_entry_with_cors() {
    let (_dir, repo_root) = setup_repo_for_read_later();
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
    let (_dir, repo_root) = setup_repo_for_read_later();
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
    let (_dir, repo_root) = setup_repo_for_read_later();
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
    let (_dir, repo_root) = setup_repo_for_read_later();
    with_server(repo_root, |port| {
        let (status, response) = http_options(port, "/api/read-later");
        assert_eq!(status, 204);
        assert_cors_headers(&response);
    });
}

#[test]
fn post_read_later_missing_url_returns_400_with_cors() {
    let (_dir, repo_root) = setup_repo_for_read_later();
    with_server(repo_root, |port| {
        let (status, body) =
            http_post_with_response(port, "/api/read-later", &json!({ "title": "no url" }));
        assert_eq!(status, 400);
        assert!(body.get("error").is_some());
    });
}

#[test]
fn patch_read_later_unknown_id_returns_404_with_cors() {
    let (_dir, repo_root) = setup_repo_for_read_later();
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
    let (_dir, repo_root) = setup_repo_with_corpus();
    with_server(repo_root, |port| {
        let (status, _) =
            http_patch(port, "/api/status", &json!({ "read": true }));
        assert_eq!(status, 405);
    });
}

#[test]
fn options_non_read_later_path_returns_405() {
    let (_dir, repo_root) = setup_repo_with_corpus();
    with_server(repo_root, |port| {
        let (status, _) = http_options(port, "/api/status");
        assert_eq!(status, 405);
    });
}
