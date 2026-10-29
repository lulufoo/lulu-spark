//! MCP Host OAuth routes: register, consent, token. One origin with the MCP server.

use std::net::{SocketAddr, TcpListener};
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use serde_json::{json, Value};

use super::*;
use crate::services::mcp_oauth::{verify_for_slot, Slot, TicketHandle};
use crate::test_support::TestSandbox;

const CURSOR_REDIRECT: &str = "cursor://anysphere.cursor-retrieval/oauth/user-lulu-spark/callback";
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

fn start() -> McpRuntimeHandle {
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral")
        .local_addr()
        .expect("local addr")
        .port();
    let bind_addr: SocketAddr = format!("127.0.0.1:{port}").parse().expect("bind addr");
    start_embedded_mcp_runtime(McpRuntimeConfig { bind_addr }).expect("start")
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("http client")
}

fn base(handle: &McpRuntimeHandle) -> String {
    format!("http://127.0.0.1:{}", handle.local_addr().port())
}

fn post_json(url: &str, body: &Value) -> Response {
    client().post(url).json(body).send().expect("POST json")
}

fn post_form(url: &str, pairs: &[(&str, &str)]) -> Response {
    let body: Vec<String> = pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
        .collect();
    client()
        .post(url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(body.join("&"))
        .send()
        .expect("POST form")
}

fn location(response: &Response) -> String {
    response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

fn register(base: &str, name: &str, redirect: &str) -> Value {
    let response = post_json(
        &format!("{base}/oauth/register"),
        &json!({ "client_name": name, "redirect_uris": [redirect] }),
    );
    assert_eq!(response.status().as_u16(), 201);
    response.json().expect("registration json")
}

fn consent_url(base: &str, client_id: &str, redirect: &str) -> String {
    format!(
        "{base}/oauth/consent?client_id={}&redirect_uri={}&code_challenge={CHALLENGE}&state=xyz",
        urlencoding::encode(client_id),
        urlencoding::encode(redirect)
    )
}

#[test]
fn authorization_server_advertises_registration_on_the_same_origin() {
    let handle = start();
    let base = base(&handle);
    let meta: Value = client()
        .get(format!("{base}/.well-known/oauth-authorization-server"))
        .send()
        .expect("GET metadata")
        .json()
        .expect("json");
    assert_eq!(meta["registration_endpoint"], format!("{base}/oauth/register"));
    assert_eq!(meta["token_endpoint_auth_methods_supported"][0], "none");
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn register_issues_a_public_client_id() {
    let handle = start();
    let base = base(&handle);
    let response = post_json(
        &format!("{base}/oauth/register"),
        &json!({
            "client_name": "Cursor",
            "redirect_uris": [CURSOR_REDIRECT],
            "token_endpoint_auth_method": "client_secret_basic",
        }),
    );
    assert_eq!(response.status().as_u16(), 201);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let body: Value = response.json().expect("json");
    assert!(body["client_id"].as_str().expect("id").starts_with("lulu-"));
    assert_eq!(body["client_name"], "Cursor");
    assert_eq!(body["redirect_uris"][0], CURSOR_REDIRECT);
    assert_eq!(body["token_endpoint_auth_method"], "none");
    assert!(body.get("client_secret").is_none(), "{body}");
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn register_rejects_unusable_metadata() {
    let handle = start();
    let url = format!("{}/oauth/register", base(&handle));
    for (body, error) in [
        (json!({ "redirect_uris": ["https://evil.example/cb"] }), "invalid_redirect_uri"),
        (json!({ "redirect_uris": ["javascript:alert(1)"] }), "invalid_redirect_uri"),
        (json!({ "redirect_uris": ["spark://auth-login/callback"] }), "invalid_redirect_uri"),
        (json!({ "redirect_uris": [] }), "invalid_redirect_uri"),
        (json!({ "redirect_uris": "cursor://a/cb" }), "invalid_redirect_uri"),
        (json!({ "redirect_uris": [7] }), "invalid_redirect_uri"),
        (json!({ "client_name": "no redirects" }), "invalid_redirect_uri"),
    ] {
        let response = post_json(&url, &body);
        assert_eq!(response.status().as_u16(), 400, "{body}");
        let json: Value = response.json().expect("json");
        assert_eq!(json["error"], error, "{body}");
    }
    let response = client()
        .post(&url)
        .body("not json")
        .send()
        .expect("POST text");
    assert_eq!(response.status().as_u16(), 400);
    let json: Value = response.json().expect("json");
    assert_eq!(json["error"], "invalid_client_metadata");
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn oauth_routes_do_not_call_other_hosts() {
    for (name, source) in [
        ("oauth_grant", include_str!("../mcp_host/server/oauth_grant.rs")),
        ("oauth_register", include_str!("../mcp_host/server/oauth_register.rs")),
        ("oauth_metadata", include_str!("../mcp_host/server/oauth_metadata.rs")),
    ] {
        assert!(!source.contains("main_host::"), "{name} must not call Main Host");
        assert!(!source.contains("gateway::"), "{name} must not call Gateway");
        assert!(!source.contains("7654"), "{name} must not name Gateway 7654");
    }
}

#[test]
fn oauth_is_served_by_the_mcp_host_only() {
    let gateway = include_str!("../gateway/mod.rs");
    let main_host = include_str!("../main_host/dispatch.rs");
    for (name, source) in [("Gateway", gateway), ("Main Host", main_host)] {
        assert!(!source.contains("/oauth/"), "{name} must not serve OAuth routes");
        assert!(!source.contains("/.well-known/"), "{name} must not serve OAuth metadata");
    }
}

#[path = "mcp_host_oauth/consent.rs"]
mod consent;
