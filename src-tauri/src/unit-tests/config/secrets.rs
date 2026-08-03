use super::*;

#[test]
fn set_and_get_github_token() {
    test_secrets_clear();
    set_secret(KEY_GITHUB_TOKEN, "pat-test").expect("set");
    assert_eq!(
        get_secret(KEY_GITHUB_TOKEN).expect("get"),
        Some("pat-test".to_string())
    );
}

#[test]
fn empty_github_token_deletes_entry() {
    test_secrets_clear();
    set_secret(KEY_GITHUB_TOKEN, "x").expect("set");
    delete_secret(KEY_GITHUB_TOKEN).expect("del");
    assert_eq!(get_secret(KEY_GITHUB_TOKEN).expect("get"), None);
}

#[test]
fn has_llm_key_false_when_unset() {
    test_secrets_clear();
    assert!(!has_llm_key());
}

#[test]
fn apply_token_payload_stores_llm_api_key() {
    test_secrets_clear();
    apply_token_payload(&serde_json::json!({ "api_key": "sk-llm-secret" })).expect("apply");
    assert!(has_llm_key());
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk-llm-secret".to_string())
    );
}

#[test]
fn apply_token_payload_empty_api_key_clears_llm_secret() {
    test_secrets_clear();
    set_secret(KEY_LLM_API_KEY, "sk-old").expect("set");
    apply_token_payload(&serde_json::json!({ "api_key": "" })).expect("apply");
    assert!(!has_llm_key());
    assert_eq!(get_secret(KEY_LLM_API_KEY).expect("get"), None);
}

#[test]
fn apply_token_payload_stores_per_category_keys_independently() {
    test_secrets_clear();
    apply_token_payload(&serde_json::json!({
        "api_key_host": "sk-host",
        "api_key_cursor": "sk-cursor"
    }))
    .expect("apply");
    assert!(has_host_key());
    assert!(has_cursor_key());
    assert!(has_llm_key()); // host alias
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk-host".to_string())
    );
    assert_eq!(
        get_secret(KEY_LLM_API_KEY_CURSOR).expect("get"),
        Some("sk-cursor".to_string())
    );
}

#[test]
fn apply_token_payload_empty_api_key_host_does_not_clear_existing() {
    test_secrets_clear();
    set_secret(KEY_LLM_API_KEY, "sk-keep").expect("set");
    apply_token_payload(&serde_json::json!({ "api_key_host": "" })).expect("apply");
    assert!(has_host_key());
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk-keep".to_string())
    );
}

#[test]
fn apply_token_payload_empty_api_key_cursor_does_not_clear_existing() {
    test_secrets_clear();
    set_secret(KEY_LLM_API_KEY_CURSOR, "sk-cursor-keep").expect("set");
    apply_token_payload(&serde_json::json!({ "api_key_cursor": "" })).expect("apply");
    assert!(has_cursor_key());
    assert_eq!(
        get_secret(KEY_LLM_API_KEY_CURSOR).expect("get"),
        Some("sk-cursor-keep".to_string())
    );
}

#[test]
fn migrate_legacy_llm_api_key_preserves_host_credential() {
    test_secrets_clear();
    set_secret(KEY_LLM_API_KEY, "sk-legacy-keep").expect("set");
    migrate_legacy_llm_api_key_to_host().expect("migrate");
    assert!(has_host_key());
    assert_eq!(
        get_secret(KEY_LLM_API_KEY).expect("get"),
        Some("sk-legacy-keep".to_string())
    );
}
