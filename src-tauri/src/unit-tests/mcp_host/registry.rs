use std::collections::BTreeMap;

use super::*;

fn sample_transport(label: &str) -> HttpMcpTransport {
    HttpMcpTransport {
        name: "spark".into(),
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
    assert_eq!(transport.name, "spark");
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
    let settings_src = crate::test_support::read_rs_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/config/settings"
    ));
    assert!(
        !settings_src.contains("HttpMcpTransport")
            && !settings_src.contains("mcp_http_transport")
            && !settings_src.contains("mcpServers"),
        "MCP HTTP transport must not be a user settings field"
    );
}

/// Binding business key ≡ MCP scene_slot (App global slot).
const SPARK_SCENE_SLOT: &str = "spark";

const SEEDED_SPARK_CAPABILITY: &str = "internal host-mcp spark capability surface";

fn expected_seed_url(key: &str) -> String {
    format!(
        "http://127.0.0.1:{}/mcp/{}",
        crate::DEFAULT_MCP_PORT,
        key
    )
}

fn registry_source() -> &'static str {
    include_str!("../../mcp_host/registry/mod.rs")
}

#[test]
fn seeded_business_key_is_spark() {
    assert_eq!(
        SEEDED_BUSINESS_KEY, SPARK_SCENE_SLOT,
        "sole seeded Binding key must be spark"
    );
}

#[test]
fn seed_defaults_registers_only_spark_with_mcp_spark_url() {
    clear_for_tests();
    seed_defaults();
    let got = lookup(SPARK_SCENE_SLOT).expect("spark must be seeded");
    assert!(
        !got.capability_description.trim().is_empty(),
        "spark seed must expose a non-empty decision-level description"
    );
    assert_eq!(got.capability_description, SEEDED_SPARK_CAPABILITY);
    assert_eq!(
        got.http_transport().url,
        expected_seed_url(SPARK_SCENE_SLOT),
        "spark transport URL must be http://127.0.0.1:{{port}}/mcp/spark"
    );
    assert!(
        got.http_transport().url.ends_with("/mcp/spark"),
        "HTTP URL must end with /mcp/spark: {}",
        got.http_transport().url
    );
    let last = got
        .http_transport()
        .url
        .rsplit('/')
        .next()
        .expect("url path segment");
    assert_eq!(last, SPARK_SCENE_SLOT);
}

#[test]
fn lookup_notes_and_todo_task_return_not_found() {
    clear_for_tests();
    seed_defaults();
    assert_eq!(
        lookup("notes").expect_err("notes must not remain a registry key"),
        McpServerLookupError::NotFound
    );
    assert_eq!(
        lookup("todo_task").expect_err("todo_task must not remain a registry key"),
        McpServerLookupError::NotFound
    );
}

#[test]
fn registry_no_longer_exports_seeded_notes_key() {
    let src = registry_source();
    assert!(
        !src.contains("SEEDED_NOTES_KEY"),
        "registry must no longer export SEEDED_NOTES_KEY"
    );
}

#[test]
fn seed_defaults_after_clear_reseeds_spark_and_is_idempotent() {
    clear_for_tests();
    let err = lookup(SPARK_SCENE_SLOT).expect_err("cleared spark must be absent");
    assert_eq!(err, McpServerLookupError::NotFound);
    seed_defaults();
    let first = lookup(SPARK_SCENE_SLOT).expect("reseed spark");
    seed_defaults();
    let second = lookup(SPARK_SCENE_SLOT).expect("second seed_defaults still finds spark");
    assert_eq!(first, second);
    assert_eq!(
        second.http_transport().url,
        expected_seed_url(SPARK_SCENE_SLOT)
    );
    assert_eq!(
        lookup("notes").expect_err("notes stays unregistered"),
        McpServerLookupError::NotFound
    );
    assert_eq!(
        lookup("todo_task").expect_err("todo_task stays unregistered"),
        McpServerLookupError::NotFound
    );
}

#[test]
fn unregistered_key_still_not_found_after_spark_seed() {
    clear_for_tests();
    seed_defaults();
    lookup(SPARK_SCENE_SLOT).expect("spark seeded");
    let err = lookup("unknown_business_key_xyz").expect_err("unknown key must fail");
    assert_eq!(err, McpServerLookupError::NotFound);
    let cursor = lookup("cursor_ide").expect_err("spark must not reuse cursor_ide as a Binding key");
    assert_eq!(cursor, McpServerLookupError::NotFound);
}

#[test]
fn empty_key_still_invalid_does_not_fall_to_spark() {
    clear_for_tests();
    seed_defaults();
    let err = lookup("").expect_err("empty key must fail");
    assert_eq!(err, McpServerLookupError::InvalidKey);
    let ws = lookup("   ").expect_err("whitespace key must fail");
    assert_eq!(ws, McpServerLookupError::InvalidKey);
    lookup(SPARK_SCENE_SLOT).expect("empty key must not silently land on spark");
}

#[test]
fn table_init_and_seed_defaults_only_register_spark() {
    let src = registry_source();
    let init = src.split("get_or_init").nth(1).expect("table get_or_init");
    let init_body = init.split("fn validate_key").next().expect("init body");
    assert!(
        init_body.contains("spark") || init_body.contains("SEEDED_BUSINESS_KEY"),
        "table init must seed spark on Host startup"
    );
    assert!(
        !init_body.contains("SEEDED_NOTES") && !init_body.contains("\"notes\""),
        "table init must not seed notes"
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
        seed_body.contains("SEEDED_BUSINESS_KEY") || seed_body.contains("spark"),
        "seed_defaults must register spark"
    );
    assert!(
        !seed_body.contains("SEEDED_NOTES") && !seed_body.contains("\"notes\""),
        "seed_defaults must not register notes"
    );
    let register_count = seed_body.matches("register(").count();
    assert_eq!(
        register_count, 1,
        "seed_defaults must register exactly one key (spark), got {register_count}"
    );
}
