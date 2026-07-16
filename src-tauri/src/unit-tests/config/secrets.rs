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
