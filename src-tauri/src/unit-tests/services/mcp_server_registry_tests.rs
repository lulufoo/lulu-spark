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
    // Candidate URL presence is transport data, not an endpoint readiness claim.
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

/// Binding business key ≡ MCP scene_slot (L-slot-I).
const NOTES_SCENE_SLOT: &str = "notes";

const SEEDED_TODO_CAPABILITY: &str = "internal host-mcp todo_task capability surface";

fn expected_seed_url(key: &str) -> String {
    format!(
        "http://127.0.0.1:{}/mcp/{}",
        crate::DEFAULT_MCP_PORT,
        key
    )
}

#[test]
fn lookup_notes_returns_seeded_mcp_notes_url() {
    clear_for_tests();
    seed_defaults();
    let got = lookup(NOTES_SCENE_SLOT).expect("notes must be seeded");
    assert!(
        !got.capability_description.trim().is_empty(),
        "notes seed must expose a non-empty decision-level description"
    );
    assert_eq!(
        got.http_transport().url,
        expected_seed_url(NOTES_SCENE_SLOT),
        "notes transport URL must be http://127.0.0.1:{{port}}/mcp/notes, not a bare path"
    );
    assert!(
        got.http_transport().url.contains("/mcp/notes"),
        "notes path must be /mcp/notes: {}",
        got.http_transport().url
    );
}

#[test]
fn notes_binding_key_equals_scene_slot_string() {
    clear_for_tests();
    seed_defaults();
    let got = lookup(NOTES_SCENE_SLOT).expect("notes");
    let last = got
        .http_transport()
        .url
        .rsplit('/')
        .next()
        .expect("url path segment");
    assert_eq!(
        last, NOTES_SCENE_SLOT,
        "URL last segment (scene_slot) must equal Binding key `notes`"
    );
    assert_eq!(
        NOTES_SCENE_SLOT, "notes",
        "App Binding business key and MCP scene_slot share the same string"
    );
}

#[test]
fn todo_task_seed_url_and_capability_remain_unchanged() {
    clear_for_tests();
    seed_defaults();
    let got = lookup(SEEDED_BUSINESS_KEY).expect("todo_task seed must remain");
    assert_eq!(got.capability_description, SEEDED_TODO_CAPABILITY);
    assert_eq!(
        got.http_transport().url,
        expected_seed_url(SEEDED_BUSINESS_KEY)
    );
    let notes = lookup(NOTES_SCENE_SLOT).expect("notes");
    assert_ne!(
        notes.http_transport().url,
        got.http_transport().url,
        "notes must not reuse the todo_task URL"
    );
    assert_ne!(
        notes.capability_description, got.capability_description,
        "notes must not reuse the todo_task capability description"
    );
}

#[test]
fn seed_defaults_after_clear_reseeds_notes_and_is_idempotent() {
    clear_for_tests();
    let err = lookup(NOTES_SCENE_SLOT).expect_err("cleared notes must be absent");
    assert_eq!(err, McpServerLookupError::NotFound);
    seed_defaults();
    let first = lookup(NOTES_SCENE_SLOT).expect("reseed notes");
    seed_defaults();
    let second = lookup(NOTES_SCENE_SLOT).expect("second seed_defaults still finds notes");
    assert_eq!(first, second);
    assert_eq!(
        second.http_transport().url,
        expected_seed_url(NOTES_SCENE_SLOT)
    );
}

#[test]
fn unregistered_key_still_not_found_after_notes_seed() {
    clear_for_tests();
    seed_defaults();
    lookup(NOTES_SCENE_SLOT).expect("notes seeded");
    let err = lookup("unknown_business_key_xyz").expect_err("unknown key must fail");
    assert_eq!(err, McpServerLookupError::NotFound);
    let cursor = lookup("cursor_ide").expect_err("notes must not reuse cursor_ide slot");
    assert_eq!(cursor, McpServerLookupError::NotFound);
}

#[test]
fn empty_key_still_invalid_after_notes_seed() {
    clear_for_tests();
    seed_defaults();
    let err = lookup("").expect_err("empty key must fail");
    assert_eq!(err, McpServerLookupError::InvalidKey);
}

#[test]
fn table_init_and_seed_defaults_both_register_notes() {
    let src = include_str!("../../services/mcp_server_registry.rs");
    let init = src.split("get_or_init").nth(1).expect("table get_or_init");
    let init_body = init.split("fn validate_key").next().expect("init body");
    assert!(
        init_body.contains("notes") || init_body.contains("SEEDED_NOTES"),
        "table init must seed notes on Host startup"
    );
    let seed_fn = src
        .split("pub fn seed_defaults")
        .nth(1)
        .expect("seed_defaults");
    let seed_body = seed_fn
        .split("pub fn clear_for_tests")
        .next()
        .expect("seed_defaults body");
    assert!(
        seed_body.contains("notes") || seed_body.contains("SEEDED_NOTES"),
        "seed_defaults must register notes"
    );
}
