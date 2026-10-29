//! Consent and token routes on the MCP Host, from register to an accepted MCP request.

use super::*;

#[test]
fn consent_page_names_the_registered_client_and_escapes_it() {
    let handle = start();
    let base = base(&handle);
    let cursor = register(&base, "Cursor", CURSOR_REDIRECT);
    let page = client()
        .get(consent_url(&base, cursor["client_id"].as_str().expect("id"), CURSOR_REDIRECT))
        .send()
        .expect("GET consent");
    assert_eq!(page.status().as_u16(), 200);
    let text = page.text().expect("html");
    assert!(text.contains("Allow Cursor to connect?"), "{text}");
    assert!(!text.contains("Codex"), "{text}");

    let hostile = register(&base, "<script>alert(1)</script>", CURSOR_REDIRECT);
    let text = client()
        .get(consent_url(&base, hostile["client_id"].as_str().expect("id"), CURSOR_REDIRECT))
        .send()
        .expect("GET consent")
        .text()
        .expect("html");
    assert!(!text.contains("<script>alert(1)"), "{text}");
    assert!(text.contains("&lt;script&gt;alert(1)"), "{text}");
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn consent_page_refuses_requests_it_cannot_trust() {
    let handle = start();
    let base = base(&handle);
    let cursor = register(&base, "Cursor", CURSOR_REDIRECT);
    let id = cursor["client_id"].as_str().expect("id");
    for url in [
        format!("{base}/oauth/consent"),
        consent_url(&base, id, "cursor://anysphere.cursor-retrieval/other"),
        consent_url(&base, id, "https://evil.example/cb"),
        format!(
            "{base}/oauth/consent?client_id=&redirect_uri={}",
            urlencoding::encode(CURSOR_REDIRECT)
        ),
    ] {
        let response = client().get(&url).send().expect("GET consent");
        assert_eq!(response.status().as_u16(), 400, "{url}");
        let text = response.text().expect("html");
        assert!(text.contains("Invalid authorization request"), "{text}");
        assert!(!text.contains("Allow"), "{text}");
    }
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn consent_allow_and_deny_redirect_to_the_registered_uri() {
    let handle = start();
    let base = base(&handle);
    let cursor = register(&base, "Cursor", CURSOR_REDIRECT);
    let id = cursor["client_id"].as_str().expect("id");
    let url = format!("{base}/oauth/consent");
    let allow = post_form(
        &url,
        &[
            ("decision", "allow"),
            ("client_id", id),
            ("redirect_uri", CURSOR_REDIRECT),
            ("code_challenge", CHALLENGE),
            ("state", "xyz"),
        ],
    );
    assert_eq!(allow.status().as_u16(), 302);
    let target = location(&allow);
    assert!(target.starts_with(&format!("{CURSOR_REDIRECT}?code=")), "{target}");
    assert!(target.ends_with("&state=xyz"), "{target}");

    let deny = post_form(
        &url,
        &[
            ("decision", "deny"),
            ("client_id", id),
            ("redirect_uri", CURSOR_REDIRECT),
            ("state", "xyz"),
        ],
    );
    assert_eq!(deny.status().as_u16(), 302);
    assert_eq!(
        location(&deny),
        format!("{CURSOR_REDIRECT}?error=access_denied&state=xyz")
    );
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn consent_post_never_redirects_to_an_unregistered_uri() {
    let handle = start();
    let base = base(&handle);
    let cursor = register(&base, "Cursor", CURSOR_REDIRECT);
    let id = cursor["client_id"].as_str().expect("id");
    for (decision, redirect) in [
        ("allow", "cursor://anysphere.cursor-retrieval/other"),
        ("deny", "cursor://anysphere.cursor-retrieval/other"),
        ("allow", "https://evil.example/cb"),
        ("allow", "spark://auth-login/callback"),
        ("allow", ""),
    ] {
        let response = post_form(
            &format!("{base}/oauth/consent"),
            &[
                ("decision", decision),
                ("client_id", id),
                ("redirect_uri", redirect),
                ("code_challenge", CHALLENGE),
            ],
        );
        assert_eq!(response.status().as_u16(), 400, "{decision} {redirect}");
        assert_eq!(location(&response), "", "{decision} {redirect}");
    }
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn client_forgotten_after_restart_still_works_with_a_loopback_redirect() {
    let handle = start();
    let base = base(&handle);
    let response = post_form(
        &format!("{base}/oauth/consent"),
        &[
            ("decision", "allow"),
            ("client_id", "lulu-from-before-restart"),
            ("redirect_uri", "http://127.0.0.1:1455/auth/callback"),
            ("code_challenge", CHALLENGE),
        ],
    );
    assert_eq!(response.status().as_u16(), 302);
    assert!(
        location(&response).starts_with("http://127.0.0.1:1455/auth/callback?code="),
        "{}",
        location(&response)
    );
    stop_embedded_mcp_runtime(handle).expect("stop");
}

#[test]
fn cursor_register_consent_token_then_mcp_request_is_accepted() {
    let _sandbox = TestSandbox::new();
    crate::test_support::reset_vault();
    let handle = start();
    let base = base(&handle);
    let cursor = register(&base, "Cursor", CURSOR_REDIRECT);
    let id = cursor["client_id"].as_str().expect("id");
    let resource = format!("{base}/mcp/cursor");

    let allow = post_form(
        &format!("{base}/oauth/consent"),
        &[
            ("decision", "allow"),
            ("client_id", id),
            ("redirect_uri", CURSOR_REDIRECT),
            ("code_challenge", CHALLENGE),
            ("resource", &resource),
        ],
    );
    let target = location(&allow);
    let code = target
        .split("code=")
        .nth(1)
        .and_then(|rest| rest.split('&').next())
        .expect("code in redirect");
    let exchange = |code: &str| {
        post_form(
            &format!("{base}/oauth/token"),
            &[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("code_verifier", VERIFIER),
                ("redirect_uri", CURSOR_REDIRECT),
                ("client_id", id),
            ],
        )
    };
    let token = exchange(code);
    assert_eq!(token.status().as_u16(), 200);
    let body: Value = token.json().expect("token json");
    assert_eq!(body["token_type"], "Bearer");
    assert!(body.get("refresh_token").is_none(), "{body}");
    let secret = body["access_token"].as_str().expect("access_token");

    let ticket = TicketHandle::from_secret(secret);
    verify_for_slot(Slot::CursorIde, ticket.clone()).expect("cursor ticket");
    assert!(verify_for_slot(Slot::Codex, ticket).is_err(), "not a codex ticket");
    assert_eq!(exchange(code).status().as_u16(), 400, "a code works once");

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"oauth-e2e","version":"0.0.1"}}}"#;
    let mcp = client()
        .post(&resource)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Authorization", format!("Bearer {secret}"))
        .body(init)
        .send()
        .expect("POST mcp");
    assert_eq!(mcp.status().as_u16(), 200, "token from OAuth must open /mcp/cursor");
    stop_embedded_mcp_runtime(handle).expect("stop");
}
