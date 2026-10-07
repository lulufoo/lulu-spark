use super::*;

#[test]
fn auth_login_landing_html_opens_callback_from_location_search_and_hash() {
    let html = auth_login_landing_html();
    assert!(
        html.contains("location.search"),
        "script must read location.search: {html}"
    );
    assert!(
        html.contains("location.hash"),
        "script must read location.hash: {html}"
    );
    assert!(
        html.contains("spark://auth-login/callback"),
        "script must open spark://auth-login/callback: {html}"
    );
}

#[test]
fn auth_login_landing_html_empty_search_and_hash_still_opens_scheme() {
    let html = auth_login_landing_html();
    assert!(html.contains("spark://auth-login/callback"));
    assert!(html.contains("location.search"));
    assert!(html.contains("location.hash"));
    assert!(
        !html.contains("spark://auth-login/callback?"),
        "empty search/hash must not hardcode a query onto the scheme: {html}"
    );
}

#[test]
fn production_module_declares_tests_and_does_not_bind_http() {
    let prod = include_str!("../../services/auth_login_landing.rs");
    assert!(
        prod.contains("#[path = \"../unit-tests/services/auth_login_landing.rs\"]"),
        "production file only declares that tests exist"
    );
    assert!(!prod.contains("#[test]"), "tests belong in the test tree");
    assert!(!prod.contains("Server::http"));
    assert!(!prod.contains("TcpListener"));
    assert!(!prod.contains("tiny_http"));
}
