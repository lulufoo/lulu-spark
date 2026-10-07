use std::fs;
use std::path::PathBuf;

use reqwest::blocking;
use serde_json::json;

use super::{http_post, setup_repo_with_notes, with_server, DEFAULT_HTTP_PORT};

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

fn http_post_text(port: u16, path: &str) -> (u16, String, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let response = blocking::Client::new()
        .post(&url)
        .json(&json!({}))
        .send()
        .expect("http post");
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

fn assert_landing_html(content_type: &str, body: &str) {
    assert!(
        content_type.contains("text/html"),
        "Content-Type must include text/html, got {content_type:?}"
    );
    assert!(
        body.contains("location.search"),
        "landing script must read location.search: {body}"
    );
    assert!(
        body.contains("location.hash"),
        "landing script must read location.hash: {body}"
    );
    assert!(
        body.contains("spark://auth-login/callback"),
        "landing script must open spark://auth-login/callback: {body}"
    );
}

/// Normal: GET /api/auth-login/landing returns 200 HTML that opens the scheme from the address bar.
#[test]
fn get_auth_login_landing_returns_200_html_opening_scheme_from_location() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, content_type, body) = http_get_text(port, "/api/auth-login/landing");
        assert_eq!(status, 200, "GET landing must be 200, got {status}: {body}");
        assert_landing_html(&content_type, &body);
    });
}

/// Boundary: no query and no hash still produces a page that can open the bare scheme.
#[test]
fn get_auth_login_landing_without_query_or_hash_still_opens_scheme() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, content_type, body) = http_get_text(port, "/api/auth-login/landing");
        assert_eq!(status, 200, "{body}");
        assert_landing_html(&content_type, &body);
        assert!(
            !body.contains("spark://auth-login/callback?"),
            "empty search/hash must not hardcode a query onto the scheme: {body}"
        );
    });
}

/// Boundary: `?pass=` stays on the address bar; the page still reads search/hash.
#[test]
fn get_auth_login_landing_keeps_pass_query_for_the_page_script() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, content_type, body) =
            http_get_text(port, "/api/auth-login/landing?pass=inflight-bag");
        assert_eq!(status, 200, "{body}");
        assert_landing_html(&content_type, &body);
        assert!(
            body.contains("location.search"),
            "hash never reaches the server; script must read the address bar"
        );
    });
}

/// Exception: POST must not produce landing HTML (405 or the existing unsupported-method response).
#[test]
fn post_auth_login_landing_does_not_produce_landing_html() {
    let fixture = setup_repo_with_notes();
    let repo_root = fixture.repo_root.clone();
    with_server(repo_root, |port| {
        let (status, content_type, body) = http_post_text(port, "/api/auth-login/landing");
        assert_ne!(status, 200, "POST must not succeed as landing HTML: {body}");
        assert!(
            status == 405 || status == 404,
            "POST must match existing unsupported handling (405/404), got {status}: {body}"
        );
        assert!(
            !content_type.contains("text/html"),
            "POST must not return landing HTML, got {content_type}: {body}"
        );
        assert!(
            !body.contains("spark://auth-login/callback") && !body.contains("location.search"),
            "POST must not produce the landing script: {body}"
        );
        let (json_status, json_body) = http_post(port, "/api/auth-login/landing", &json!({}));
        assert_eq!(json_status, status);
        assert!(json_body.get("error").is_some(), "{json_body}");
    });
}

/// Exception: L4 does not gain an HTTP listen; HTML is served on the existing Main Host loopback.
#[test]
fn l4_does_not_gain_http_listen_and_landing_uses_main_host() {
    assert_eq!(DEFAULT_HTTP_PORT, 8765);
    let host = fs::read_to_string(src("main_host/mod.rs")).expect("main host");
    assert!(
        host.contains("127.0.0.1:{port}"),
        "Main Host must keep the existing loopback listen"
    );
    assert!(
        !host.contains("0.0.0.0"),
        "Main Host must not bind a public interface"
    );

    let landing = fs::read_to_string(src("services/auth_login_landing.rs"))
        .expect("Host/L4 must produce landing HTML");
    assert!(
        !landing.contains("Server::http")
            && !landing.contains("TcpListener")
            && !landing.contains("tiny_http::Server"),
        "auth_login_landing must not bind HTTP"
    );

    let l4 = crate::test_support::read_rs_dir(src("services"));
    let agent = crate::test_support::read_rs_dir(src("agent"));
    for (label, text) in [("services", l4.as_str()), ("agent", agent.as_str())] {
        assert!(
            !text.contains("tiny_http::Server") && !text.contains("Server::http("),
            "L4 {label} must not gain an HTTP listen"
        );
    }
}
