use super::*;

#[test]
fn oauth_consent_html_is_a_new_page_not_the_login_landing() {
    let html = oauth_consent_html();
    assert!(html.contains("MCP authorization"), "{html}");
    assert!(html.contains("Allow Codex to connect?"), "{html}");
    assert!(html.contains(">Allow<"), "{html}");
    assert!(html.contains(">Deny<"), "{html}");
    assert!(html.contains("Lulu Spark"), "{html}");
    assert!(
        html.contains("rel=\"icon\"") && html.contains("viewBox='0 0 32 32'"),
        "tab must use the L mark as favicon: {html}"
    );
    assert!(!html.contains("/auth-login/landing"), "{html}");
    assert!(!html.contains("spark://auth-login/callback"), "{html}");
    assert!(!html.contains("Authorization successful"), "{html}");
    assert!(!html.contains("Desktop sign-in"), "{html}");
}

#[test]
fn oauth_consent_html_embeds_sheet_and_behavior() {
    let html = oauth_consent_html();
    assert!(html.contains("--paper: #f7f8fc"), "{html}");
    assert!(html.contains("getElementById(\"allow\")"), "{html}");
    assert!(!html.contains("<style id=\"sheet\"></style>"), "{html}");
    assert!(!html.contains("<script id=\"behavior\"></script>"), "{html}");
}

#[test]
fn production_module_declares_tests_and_does_not_bind_http() {
    let prod = include_str!("../../services/oauth_consent.rs");
    assert!(
        prod.contains("#[path = \"../unit-tests/services/oauth_consent.rs\"]"),
        "production file only declares that tests exist"
    );
    assert!(!prod.contains("#[test]"), "tests belong in the test tree");
    assert!(!prod.contains("Server::http"));
    assert!(!prod.contains("TcpListener"));
    assert!(!prod.contains("tiny_http"));
    assert!(!prod.contains("/oauth/consent"));
}

#[test]
fn oauth_consent_html_names_the_client_everywhere() {
    let html = oauth_consent_html_for("Cursor");
    assert!(html.contains("Allow Cursor to connect?"), "{html}");
    assert!(html.contains("Cursor wants to use Lulu Spark"), "{html}");
    assert!(html.contains("<dd>Cursor</dd>"), "{html}");
    assert!(!html.contains("Codex"), "{html}");
    assert!(!html.contains("{{client}}"), "{html}");
}

#[test]
fn oauth_consent_html_escapes_an_untrusted_client_name() {
    let html = oauth_consent_html_for("<img src=x onerror=alert(1)>\"'&");
    assert!(!html.contains("<img src=x"), "{html}");
    assert!(
        html.contains("&lt;img src=x onerror=alert(1)&gt;&quot;&#39;&amp;"),
        "{html}"
    );
}

#[test]
fn consent_script_posts_the_decision_back_and_never_uses_spark() {
    let js = include_str!("../../services/oauth_consent.js");
    assert!(js.contains("form.action = \"/oauth/consent\""), "{js}");
    for field in ["decision", "redirect_uri", "client_id", "code_challenge", "state", "resource"] {
        assert!(js.contains(&format!("add(\"{field}\"")), "missing {field}: {js}");
    }
    assert!(!js.contains("spark://"), "{js}");
}
