use super::*;

use crate::test_support::TestSandbox;

/// LLM slot only. Parallel lib tests share vault scope `_default`; wiping MCP
/// tickets here races mcp_host / oauth (HTTP 401).
pub fn test_secrets_clear() {
    crate::config::vault::test_clear_llm();
}

#[test]
fn sandbox_secret_roundtrip_stays_in_memory() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    set_secret("host", "sk-sandbox").expect("set");
    assert_eq!(
        get_secret("host").expect("get"),
        Some("sk-sandbox".to_string())
    );
    assert!(crate::config::settings::uses_in_memory_keychain().expect("mode"));
    let src = include_str!("../../config/secrets.rs");
    assert!(!src.contains("uses_in_memory_keychain"));
    assert!(!src.contains("key != KEY_LLM_API_KEY"));
}

#[test]
fn github_token_slot_is_gone() {
    let _sandbox = TestSandbox::new();
    let src = include_str!("../../config/secrets.rs");
    assert!(
        !src.contains("KEY_GITHUB_TOKEN")
            && !src.contains("github_token")
            && !src.contains("has_github_token"),
        "github_token must not remain in secrets.rs"
    );
    test_secrets_clear();
    apply_token_payload(&serde_json::json!({ "github_token": "ghp_leftover" }))
        .expect("apply");
    assert!(list_saved_engines().is_empty());
}

#[test]
fn keys_are_stored_per_engine_and_do_not_overwrite_each_other() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    set_secret("openai", "sk-openai").expect("set openai");
    set_secret("claude", "sk-claude").expect("set claude");
    set_secret("openai", "sk-openai-2").expect("replace openai");

    assert_eq!(
        get_secret("openai").expect("get openai"),
        Some("sk-openai-2".to_string())
    );
    assert_eq!(
        get_secret("claude").expect("get claude"),
        Some("sk-claude".to_string())
    );
    assert_eq!(list_saved_engines(), vec!["openai", "claude"]);
}

#[test]
fn engine_without_its_own_key_does_not_fall_back_to_another_engine() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    set_secret("openai", "sk-openai").expect("set");
    assert_eq!(get_secret("host").expect("get host"), None);
    assert_eq!(get_secret("qwen").expect("get qwen"), None);
}

#[test]
fn payload_writes_only_the_named_engine() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    set_secret("host", "sk-host").expect("set host");
    apply_token_payload(&serde_json::json!({
        "llm_api_key": { "engine": "kimi", "value": "sk-kimi" }
    }))
    .expect("apply");

    assert_eq!(
        get_secret("kimi").expect("get kimi"),
        Some("sk-kimi".to_string())
    );
    assert_eq!(
        get_secret("host").expect("get host"),
        Some("sk-host".to_string())
    );
}

#[test]
fn empty_value_keeps_existing_key_when_ui_omits_credentials() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    set_secret("host", "sk-keep").expect("set");
    apply_token_payload(&serde_json::json!({
        "llm_api_key": { "engine": "host", "value": "" }
    }))
    .expect("apply");
    assert_eq!(
        get_secret("host").expect("get"),
        Some("sk-keep".to_string())
    );
}

#[test]
fn invalid_or_missing_engine_is_rejected_without_writing() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    assert!(matches!(
        set_secret("cursor", "sk-x"),
        Err(SecretError::InvalidEngine(_))
    ));
    assert!(matches!(
        apply_token_payload(&serde_json::json!({
            "llm_api_key": { "engine": "cursor", "value": "sk-x" }
        })),
        Err(SecretError::InvalidEngine(_))
    ));
    assert!(matches!(
        apply_token_payload(&serde_json::json!({
            "llm_api_key": { "value": "sk-x" }
        })),
        Err(SecretError::InvalidEngine(_))
    ));
    assert!(list_saved_engines().is_empty());
}

#[test]
fn legacy_flat_payload_fields_are_ignored() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    apply_token_payload(&serde_json::json!({
        "api_key_host": "sk-legacy-host",
        "api_key_cursor": "sk-legacy-cursor",
        "api_key": "sk-legacy"
    }))
    .expect("apply");
    assert!(list_saved_engines().is_empty());
}
