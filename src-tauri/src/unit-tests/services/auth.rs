use std::time::Duration;

use super::*;
use crate::services::mcp_oauth::{self, TicketHandle};

fn reset_codex() {
    crate::config::vault::test_clear_store_fail();
    mcp_oauth::revoke_for_slot(Slot::Codex).expect("revoke codex");
}

fn verifier() -> &'static str {
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJK"
}

fn challenge(verifier: &str) -> String {
    s256(verifier)
}

fn issue_codex(auth: &Auth, ttl: Option<Duration>) -> String {
    let mut issue = CodeIssue::s256(
        "codex",
        "http://127.0.0.1/callback",
        challenge(verifier()),
        Slot::Codex,
    );
    if let Some(ttl) = ttl {
        issue = issue.with_ttl(ttl);
    }
    auth.issue_code(issue).expect("issue code")
}

fn token_req(code: &str, verifier: &str) -> TokenRequest {
    TokenRequest {
        grant_type: "authorization_code".into(),
        code: code.into(),
        code_verifier: verifier.into(),
        redirect_uri: "http://127.0.0.1/callback".into(),
        client_id: "codex".into(),
    }
}

#[test]
fn issue_code_stores_challenge_and_token_exchanges_with_pkce() {
    reset_codex();
    let auth = Auth::new();
    let code = issue_codex(&auth, None);
    let grant = auth
        .exchange_token(token_req(&code, verifier()))
        .expect("token");
    assert_eq!(grant.token_type, "Bearer");
    assert!(!grant.access_token.is_empty());
    mcp_oauth::verify_for_slot(
        Slot::Codex,
        TicketHandle::from_secret(grant.access_token.clone()),
    )
    .expect("vault ticket");
    let body = token_body(&grant);
    assert_eq!(body["token_type"], "Bearer");
    assert!(body.get("refresh_token").is_none());
}

#[test]
fn token_rejects_wrong_verifier_and_consumed_code() {
    reset_codex();
    let auth = Auth::new();
    let code = issue_codex(&auth, None);
    let bad = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJL";
    assert_eq!(
        auth.exchange_token(token_req(&code, bad)).expect_err("pkce"),
        AuthError::invalid_grant
    );
    assert_eq!(
        auth.exchange_token(token_req(&code, verifier()))
            .expect_err("replay"),
        AuthError::invalid_grant
    );
}

#[test]
fn token_rejects_expired_code_and_foreign_grant() {
    reset_codex();
    let auth = Auth::new();
    let code = issue_codex(&auth, Some(Duration::ZERO));
    assert_eq!(
        auth.exchange_token(token_req(&code, verifier()))
            .expect_err("expired"),
        AuthError::invalid_grant
    );
    let live = issue_codex(&auth, None);
    let mut req = token_req(&live, verifier());
    req.grant_type = "refresh_token".into();
    assert_eq!(
        auth.exchange_token(req).expect_err("grant"),
        AuthError::unsupported_grant
    );
}

#[test]
fn token_rejects_redirect_or_client_mismatch() {
    reset_codex();
    let auth = Auth::new();
    let code = issue_codex(&auth, None);
    let mut req = token_req(&code, verifier());
    req.redirect_uri = "http://127.0.0.1/other".into();
    assert_eq!(
        auth.exchange_token(req).expect_err("redirect"),
        AuthError::invalid_grant
    );
}

#[test]
fn production_module_declares_tests_and_is_not_an_http_host() {
    let prod = include_str!("../../services/auth/mod.rs");
    assert!(
        prod.contains("#[path = \"../../unit-tests/services/auth.rs\"]"),
        "production file only declares that tests exist"
    );
    assert!(!prod.contains("#[test]"), "tests belong in the test tree");
    assert!(!prod.contains("Server::http"));
    assert!(!prod.contains("TcpListener"));
    assert!(!prod.contains("tiny_http"));
    assert!(!prod.contains("9876"));
    assert!(!prod.contains("set_mcp_slot"));
}
