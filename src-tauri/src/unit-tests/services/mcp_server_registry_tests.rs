use std::collections::BTreeMap;

use super::*;

fn sample_transport(label: &str) -> HttpMcpTransport {
    HttpMcpTransport {
        name: "workbench".into(),
        url: format!("http://127.0.0.1:9876/mcp/{label}"),
        headers: BTreeMap::from([(
            "Accept".into(),
            "application/json, text/event-stream".into(),
        )]),
    }
}

fn sample_config(label: &str) -> McpServerConfig {
    McpServerConfig {
        // Decision-level connection/capability description.
        capability_description: format!("mcp capability for {label}"),
        // Structured HTTP transport for SDK consumption (not readiness).
        http_transport: sample_transport(label),
    }
}

#[test]
fn lookup_registered_key_returns_non_empty_decision_level_config() {
    clear_for_tests();
    let key = "todo_task";
    let cfg = sample_config(key);
    register(key, cfg.clone()).expect("register");
    let got = lookup(key).expect("lookup registered key");
    assert!(
        !got.capability_description.trim().is_empty(),
        "config must expose a non-empty decision-level description"
    );
    assert_eq!(got, cfg);
}

#[test]
fn lookup_same_key_is_stable() {
    clear_for_tests();
    let key = "todo_task";
    register(key, sample_config(key)).expect("register");
    let a = lookup(key).expect("first lookup");
    let b = lookup(key).expect("second lookup");
    assert_eq!(a, b);
}

#[test]
fn seeded_business_key_is_registered_and_lookupable() {
    clear_for_tests();
    seed_defaults();
    let got = lookup(SEEDED_BUSINESS_KEY).expect("seeded business key must be lookupable");
    assert!(
        !got.capability_description.trim().is_empty(),
        "seeded config must be non-empty decision-level description"
    );
}

#[test]
fn unknown_key_returns_explicit_not_found() {
    clear_for_tests();
    seed_defaults();
    let err = lookup("unknown_business_key_xyz").expect_err("unknown key must fail");
    assert_eq!(err, McpServerLookupError::NotFound);
}

#[test]
fn empty_key_returns_explicit_invalid_key() {
    clear_for_tests();
    let err = lookup("").expect_err("empty key must fail");
    assert_eq!(err, McpServerLookupError::InvalidKey);
    let reg_err = register("", sample_config("x")).expect_err("empty register key must fail");
    assert_eq!(reg_err, McpServerLookupError::InvalidKey);
}

#[test]
fn whitespace_only_key_returns_explicit_invalid_key() {
    clear_for_tests();
    let err = lookup("   ").expect_err("whitespace key must fail");
    assert_eq!(err, McpServerLookupError::InvalidKey);
}

#[test]
fn config_exposes_capability_description_and_http_transport() {
    clear_for_tests();
    let cfg = sample_config("shape");
    assert!(!cfg.capability_description.is_empty());
    let transport = cfg.http_transport();
    assert_eq!(transport.name, "workbench");
    assert!(transport.url.starts_with("http://"));
    assert!(
        transport.headers.contains_key("Accept"),
        "HTTP transport must carry required Accept header"
    );
    register("shape_key", cfg.clone()).expect("register");
    let got = lookup("shape_key").expect("lookup");
    assert_eq!(got.capability_description, cfg.capability_description);
    assert_eq!(got.http_transport(), cfg.http_transport());
}

#[test]
fn seeded_config_exposes_structured_http_transport() {
    clear_for_tests();
    seed_defaults();
    let got = lookup(SEEDED_BUSINESS_KEY).expect("seeded");
    let t = got.http_transport();
    assert!(!t.name.trim().is_empty(), "transport name required");
    assert!(
        t.url.contains("/mcp"),
        "seeded transport URL must point at MCP path: {}",
        t.url
    );
    // Candidate URL presence is not readiness — that is mcp_endpoint_readiness's job.
}

#[test]
fn seeded_lookup_url_uses_mcp_key_path_slot_not_bare_mcp() {
    clear_for_tests();
    seed_defaults();
    let got = lookup(SEEDED_BUSINESS_KEY).expect("seeded");
    let expected = format!(
        "http://127.0.0.1:{}/mcp/{}",
        crate::DEFAULT_MCP_PORT,
        SEEDED_BUSINESS_KEY
    );
    assert_eq!(
        got.http_transport().url, expected,
        "registry seed/lookup must use /mcp/<key>, not bare /mcp"
    );
    // Regression: bare /mcp must fail this assertion (key≡scene_slot).
    assert!(
        !got.http_transport().url.ends_with("/mcp"),
        "must not seed bare /mcp: {}",
        got.http_transport().url
    );
    let last = got
        .http_transport()
        .url
        .rsplit('/')
        .next()
        .expect("url path segment");
    assert_eq!(
        last, SEEDED_BUSINESS_KEY,
        "URL path last segment must equal Binding key"
    );
}

#[test]
fn http_transport_is_not_a_user_setting_surface() {
    // Transport lives on Host registry config, not settings.
    let settings_src = include_str!("../../config/settings.rs");
    assert!(
        !settings_src.contains("HttpMcpTransport")
            && !settings_src.contains("mcp_http_transport")
            && !settings_src.contains("mcpServers"),
        "MCP HTTP transport must not be a user settings field"
    );
}
