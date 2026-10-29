use super::*;
use crate::services::app_log::token_ok;

#[test]
fn business_id_is_a_token_app_log_accepts() {
    assert_eq!(BUSINESS, "mcp_oauth");
    assert!(token_ok(BUSINESS));
    // A space would make `app_log` drop every line, which is why "MCP OAuth" is not used.
    assert!(!token_ok("MCP OAuth"));
}

#[test]
fn trace_follows_the_client_id_and_falls_back_to_a_fresh_one() {
    let client_id = "lulu-CQjj95opWSShm-_CIgfHZJ";
    assert_eq!(trace_for(client_id), client_id);

    for bad in ["", "short", "has space in it", "a/b/c/d/e/f/g/h"] {
        let trace = trace_for(bad);
        assert!(trace.starts_with("trace_"), "{bad:?} -> {trace}");
        assert!(parse_trace_id(&trace).is_some());
    }
}

#[test]
fn clip_drops_control_characters_and_caps_length() {
    assert_eq!(clip("Cursor/3.27\n\r\t(darwin)\u{1b}[31m"), "Cursor/3.27(darwin)[31m");
    let long = "a".repeat(FIELD_MAX + 50);
    assert_eq!(clip(&long).chars().count(), FIELD_MAX);
    assert_eq!(clip(""), "");
}

#[test]
fn production_files_log_under_this_business_and_never_log_secrets() {
    let prod = include_str!("../../services/mcp_oauth_log.rs");
    assert!(prod.contains("#[path = \"../unit-tests/services/mcp_oauth_log.rs\"]"));
    assert!(!prod.contains("#[test]"));
    assert!(prod.contains("app_log::log(BUSINESS"));

    let logging_files = [
        include_str!("../../services/auth/audit.rs"),
        include_str!("../../mcp_host/server/oauth_grant.rs"),
        include_str!("../../mcp_host/server/oauth_register.rs"),
        include_str!("../../mcp_host/server/oauth_metadata.rs"),
    ];
    // Keys that would carry a secret. `code_challenge_method` / `has_code_challenge`
    // are fine: they say whether PKCE was sent, not what it was.
    let secret_keys = [
        "\"code\":",
        "\"state\":",
        "\"code_challenge\":",
        "\"code_verifier\":",
        "\"access_token\":",
        "\"secret\":",
    ];
    for source in logging_files {
        assert!(source.contains("mcp_oauth_log"), "file must log through the helper");
        assert!(!source.contains("app_log::log"), "use the mcp_oauth_log helper");
        for key in secret_keys {
            assert!(!source.contains(key), "log field {key} must not appear");
        }
    }
}
