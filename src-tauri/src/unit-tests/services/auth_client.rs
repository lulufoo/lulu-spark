use super::*;
use crate::services::auth::Auth;

fn uris(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn redirect_uri_accepts_loopback_and_private_use_schemes() {
    for ok in [
        "http://127.0.0.1:1455/auth/callback",
        "http://localhost:8080/cb",
        "http://127.0.0.1/callback",
        "cursor://anysphere.cursor-retrieval/oauth/user-lulu-spark/callback",
        "com.example.app:/oauth2redirect",
    ] {
        assert!(valid_redirect_uri(ok), "{ok}");
    }
}

#[test]
fn redirect_uri_rejects_remote_hosts_and_dangerous_schemes() {
    for bad in [
        "",
        "https://evil.example/cb",
        "http://127.0.0.1@evil.example/cb",
        "http://127.0.0.1.evil.example/cb",
        "http://127.0.0.1:abc/cb",
        "http:127.0.0.1/cb",
        "spark://auth-login/callback",
        "SPARK://auth-login/callback",
        "javascript:alert(1)",
        "data:text/html,hi",
        "file:///etc/passwd",
        "cursor://host/cb#fragment",
        "cursor://host/a b",
        "no-scheme",
        "1bad://x",
    ] {
        assert!(!valid_redirect_uri(bad), "{bad}");
    }
}

#[test]
fn registration_issues_a_distinct_client_id_each_time() {
    let auth = Auth::new();
    let list = uris(&["http://127.0.0.1:1455/cb"]);
    let a = auth.register_client("Codex", &list).expect("first");
    let b = auth.register_client("Codex", &list).expect("second");
    assert!(a.client_id.starts_with("lulu-"), "{}", a.client_id);
    assert_ne!(a.client_id, b.client_id);
    assert_eq!(a.client_name, "Codex");
    assert_eq!(a.redirect_uris, list);
}

#[test]
fn registration_rejects_missing_or_bad_redirects() {
    let auth = Auth::new();
    assert_eq!(
        auth.register_client("x", &[]).unwrap_err(),
        AuthError::invalid_redirect_uri
    );
    assert_eq!(
        auth.register_client("x", &uris(&["https://evil.example/cb"]))
            .unwrap_err(),
        AuthError::invalid_redirect_uri
    );
    let many: Vec<String> = (0..9).map(|i| format!("cursor://a/{i}")).collect();
    assert_eq!(
        auth.register_client("x", &many).unwrap_err(),
        AuthError::invalid_redirect_uri
    );
}

#[test]
fn registration_drops_duplicate_redirects() {
    let auth = Auth::new();
    let client = auth
        .register_client("x", &uris(&["cursor://a/cb", "cursor://a/cb"]))
        .expect("register");
    assert_eq!(client.redirect_uris, uris(&["cursor://a/cb"]));
}

#[test]
fn display_name_is_trimmed_and_capped_with_a_fallback() {
    assert_eq!(display_name("  Cursor\n"), "Cursor");
    assert_eq!(display_name(""), FALLBACK_NAME);
    assert_eq!(display_name("\u{0007}\u{0000}"), FALLBACK_NAME);
    assert_eq!(display_name(&"n".repeat(500)).chars().count(), MAX_NAME_CHARS);
}

#[test]
fn registered_client_must_use_a_registered_redirect_exactly() {
    let auth = Auth::new();
    let client = auth
        .register_client("Cursor", &uris(&["cursor://a/cb"]))
        .expect("register");
    assert_eq!(
        auth.authorize_client(&client.client_id, "cursor://a/cb")
            .expect("registered redirect"),
        "Cursor"
    );
    for other in ["cursor://a/cb2", "cursor://a/cb/", "http://127.0.0.1:1/cb"] {
        assert_eq!(
            auth.authorize_client(&client.client_id, other).unwrap_err(),
            AuthError::invalid_request,
            "{other}"
        );
    }
}

#[test]
fn unknown_client_is_accepted_only_with_a_valid_redirect() {
    let auth = Auth::new();
    assert_eq!(
        auth.authorize_client("forgotten-after-restart", "cursor://a/cb")
            .expect("valid redirect"),
        FALLBACK_NAME
    );
    for bad in ["https://evil.example/cb", "javascript:alert(1)", ""] {
        assert_eq!(
            auth.authorize_client("forgotten-after-restart", bad)
                .unwrap_err(),
            AuthError::invalid_request,
            "{bad}"
        );
    }
    assert_eq!(
        auth.authorize_client("", "cursor://a/cb").unwrap_err(),
        AuthError::invalid_request
    );
}

#[test]
fn registry_drops_the_oldest_client_when_full() {
    let mut clients = Vec::new();
    for i in 0..(MAX_CLIENTS + 5) {
        let client = RegisteredClient {
            client_id: format!("c{i}"),
            client_name: "x".into(),
            redirect_uris: uris(&["cursor://a/cb"]),
        };
        push_bounded(&mut clients, client);
    }
    assert_eq!(clients.len(), MAX_CLIENTS);
    assert_eq!(clients.first().expect("first").client_id, "c5");
    assert_eq!(
        clients.last().expect("last").client_id,
        format!("c{}", MAX_CLIENTS + 4)
    );
}
