use std::fs;
use std::path::PathBuf;

use reqwest::blocking;
use serde_json::json;

use super::{setup_repo_with_notes, with_server};

fn src(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src").join(rel)
}

fn http_get_text(port: u16, path: &str) -> (u16, String, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let response = blocking::get(&url).expect("http get");
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.text().unwrap_or_default();
    (status, content_type, body)
}

fn http_post_text(port: u16, path: &str) -> (u16, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let response = blocking::Client::new()
        .post(&url)
        .json(&json!({}))
        .send()
        .expect("http post");
    (response.status().as_u16(), response.text().unwrap_or_default())
}

#[test]
fn get_oauth_consent_returns_dedicated_page_not_login_landing() {
    let fixture = setup_repo_with_notes();
    with_server(fixture.repo_root.clone(), |port| {
        let (status, content_type, body) = http_get_text(port, "/oauth/consent");
        assert_eq!(status, 200, "{body}");
        assert!(content_type.contains("text/html"), "{content_type}");
        assert!(body.contains("Allow Codex to connect?"), "{body}");
        assert!(body.contains("MCP authorization"), "{body}");
        assert!(!body.contains("/auth-login/landing"), "{body}");
        assert!(!body.contains("Authorization successful"), "{body}");
        assert!(!body.contains("spark://auth-login/callback"), "{body}");
    });
}

#[test]
fn post_oauth_consent_does_not_return_the_page() {
    let fixture = setup_repo_with_notes();
    with_server(fixture.repo_root.clone(), |port| {
        let (status, body) = http_post_text(port, "/oauth/consent");
        assert_ne!(status, 200, "{body}");
        assert!(!body.contains("Allow Codex to connect?"), "{body}");
    });
}

#[test]
fn gateway_does_not_gain_oauth_consent_path() {
    let gateway = fs::read_to_string(src("gateway/mod.rs")).expect("gateway");
    assert!(
        !gateway.contains("/oauth/consent"),
        "L0 must not add /oauth/consent"
    );
}

#[test]
fn oauth_consent_handler_does_not_bind_http() {
    let handler = fs::read_to_string(src("main_host/oauth_consent.rs")).expect("handler");
    assert!(!handler.contains("Server::http"));
    assert!(!handler.contains("TcpListener"));
}
