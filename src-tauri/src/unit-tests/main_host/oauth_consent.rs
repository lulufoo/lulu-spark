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

fn http_post_form_no_follow(port: u16, path: &str, body: &str) -> (u16, String, String) {
    let url = format!("http://127.0.0.1:{port}{path}");
    let response = blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("client")
        .post(&url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body.to_string())
        .send()
        .expect("http post");
    let status = response.status().as_u16();
    let location = response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let body = response.text().unwrap_or_default();
    (status, location, body)
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

#[test]
fn allow_posts_302_code_to_loopback_redirect_not_spark() {
    let fixture = setup_repo_with_notes();
    with_server(fixture.repo_root.clone(), |port| {
        let body = concat!(
            "decision=allow",
            "&redirect_uri=http%3A%2F%2F127.0.0.1%3A1455%2Fauth%2Fcallback",
            "&client_id=codex",
            "&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
            "&state=xyz"
        );
        let (status, location, text) = http_post_form_no_follow(port, "/oauth/consent", body);
        assert_eq!(status, 302, "{text}");
        assert!(
            location.starts_with("http://127.0.0.1:1455/auth/callback?code="),
            "{location}"
        );
        assert!(location.contains("&state=xyz"), "{location}");
        assert!(!location.contains("spark://"), "{location}");
        assert!(!text.contains("spark://"), "{text}");
    });
}

#[test]
fn deny_posts_302_access_denied_to_loopback_redirect() {
    let fixture = setup_repo_with_notes();
    with_server(fixture.repo_root.clone(), |port| {
        let body = concat!(
            "decision=deny",
            "&redirect_uri=http%3A%2F%2F127.0.0.1%3A1455%2Fauth%2Fcallback",
            "&state=xyz"
        );
        let (status, location, text) = http_post_form_no_follow(port, "/oauth/consent", body);
        assert_eq!(status, 302, "{text}");
        assert_eq!(
            location,
            "http://127.0.0.1:1455/auth/callback?error=access_denied&state=xyz"
        );
        assert!(!location.contains("spark://"), "{location}");
    });
}

#[test]
fn allow_rejects_spark_redirect() {
    let fixture = setup_repo_with_notes();
    with_server(fixture.repo_root.clone(), |port| {
        let body = concat!(
            "decision=allow",
            "&redirect_uri=spark%3A%2F%2Fauth-login%2Fcallback",
            "&client_id=codex",
            "&code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let (status, location, text) = http_post_form_no_follow(port, "/oauth/consent", body);
        assert_eq!(status, 400, "{text}");
        assert!(location.is_empty(), "{location}");
        assert!(!text.contains("spark://"), "{text}");
    });
}

#[test]
fn consent_script_posts_decision_and_never_uses_spark() {
    let js = fs::read_to_string(src("services/oauth_consent.js")).expect("js");
    assert!(js.contains("form.action = \"/oauth/consent\""), "{js}");
    assert!(js.contains("add(\"decision\", decision)"), "{js}");
    assert!(js.contains("add(\"redirect_uri\""), "{js}");
    assert!(!js.contains("spark://"), "{js}");
}
