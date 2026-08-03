use super::*;

fn sample_config(label: &str) -> McpServerConfig {
    McpServerConfig {
        // Decision-level connection/capability description only.
        // Field-level transport schema (stdio/http/…) is intentionally deferred.
        capability_description: format!("mcp capability for {label}"),
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
fn config_exposes_only_decision_level_capability_description() {
    clear_for_tests();
    let cfg = sample_config("shape");
    // Decision-level shape: a non-empty capability/connection description.
    // Transport fields (command/args/url/stdio/http) are not part of this contract.
    assert!(!cfg.capability_description.is_empty());
    register("shape_key", cfg.clone()).expect("register");
    let got = lookup("shape_key").expect("lookup");
    assert_eq!(got.capability_description, cfg.capability_description);
}
