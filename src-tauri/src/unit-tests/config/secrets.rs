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
