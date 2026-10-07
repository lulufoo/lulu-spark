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
fn auth_login_landing_html_shows_intro_success_and_close_hint() {
    let html = auth_login_landing_html();
    assert!(html.contains("Lulu Spark"), "{html}");
    assert!(html.contains("Authorization successful"), "{html}");
    assert!(html.contains("Open Lulu Spark"), "{html}");
    assert!(
        html.contains("Opening Lulu Spark. You can close this page afterwards."),
        "{html}"
    );
    assert!(!html.contains("Tried to open Lulu Spark"), "{html}");
    assert!(
        !html.contains("You can close this page after the app opens."),
        "{html}"
    );
    assert!(
        !html.contains("If the app does not open, use the button above."),
        "{html}"
    );
    assert!(!html.contains("个人知识档案与桌面助手"), "{html}");
    assert!(html.contains("window.close"), "{html}");
    assert!(
        html.contains("rel=\"icon\"") && html.contains("viewBox='0 0 32 32'"),
        "tab must use the L mark as favicon: {html}"
    );
    assert!(
        html.contains("iframe") && html.contains("frame.src = scheme"),
        "keep the visible page; open scheme without replacing it: {html}"
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
