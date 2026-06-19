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

fn with_server<F: FnOnce(u16)>(repo_root: PathBuf, f: F) {
    let port = ephemeral_port();
    let handle = start(repo_root, port).expect("start server");
    thread::sleep(Duration::from_millis(50));
    f(port);
    stop(handle);
    crate::config::settings::set_test_config_dir(None);
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

#[test]
fn unknown_path_returns_404() {
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
