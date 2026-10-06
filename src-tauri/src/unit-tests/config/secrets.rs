use super::*;

use crate::test_support::TestSandbox;

#[test]
fn sandbox_secret_roundtrip_stays_in_memory() {
    let _sandbox = TestSandbox::new();
    test_secrets_clear();
    set_secret(KEY_LLM_API_KEY, "sk-sandbox").expect("set");
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk-sandbox".to_string())
    );
    assert!(crate::config::settings::uses_in_memory_keychain());
    let src = include_str!("../../config/secrets.rs");
    assert!(src.contains("uses_in_memory_keychain"));
}

#[test]
fn github_token_slot_is_gone() {
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
    assert_eq!(get_secret("github_token").expect("get"), None);
}

#[test]
fn host_llm_key_is_the_only_assistant_credential_slot() {
    test_secrets_clear();
    apply_token_payload(&serde_json::json!({
        "api_key_host": "sk-host",
        "api_key_cursor": "sk-legacy-ignored"
    }))
    .expect("apply");

    assert!(has_host_key());
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get host"),
        Some("sk-host".to_string())
    );
}

#[test]
fn legacy_cursor_payload_does_not_create_a_credential_slot() {
    test_secrets_clear();
    apply_token_payload(&serde_json::json!({ "api_key_cursor": "sk-legacy" }))
        .expect("apply");
    assert!(!has_host_key());
    assert_eq!(
        get_secret("llm_api_key_cursor").expect("get legacy key"),
        None
    );
}

#[test]
fn empty_host_key_keeps_existing_key_when_ui_omits_credentials() {
    test_secrets_clear();
    set_secret(KEY_LLM_API_KEY, "sk-keep").expect("set");
    apply_token_payload(&serde_json::json!({ "api_key_host": "" })).expect("apply");
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk-keep".to_string())
    );
}

#[test]
fn legacy_api_key_payload_is_ignored() {
    test_secrets_clear();
    set_secret(KEY_LLM_API_KEY, "sk-existing").expect("set");
    apply_token_payload(&serde_json::json!({ "api_key": "sk-legacy" })).expect("apply");
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk-existing".to_string())
    );
}
