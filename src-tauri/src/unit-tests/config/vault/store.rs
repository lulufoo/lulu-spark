use super::{
    delete_auth_session, get_auth_session, io_lock, memory_fail_map, memory_vault, read_vault,
    scope_id, set_auth_session, store_memory_vault, update_vault, ACCOUNT_VAULT, KEYCHAIN_SERVICE,
};
use crate::config::vault::{
    AuthSession, AuthUser, BindDevice, SecretError, SlotTicket, Vault,
};

fn sample_auth_session() -> AuthSession {
    AuthSession {
        access_token: "access-aaa".into(),
        refresh_token: "refresh-bbb".into(),
        expires_at: 1_800_000_000,
        user: AuthUser {
            id: "user-42".into(),
            email: Some("ada@example.com".into()),
            name: Some("Ada".into()),
            avatar: Some("https://example.com/a.png".into()),
            provider: Some("google".into()),
        },
    }
}

pub fn test_clear_scope() {
    let _guard = io_lock();
    store_memory_vault(&Vault::default());
    memory_fail_map().remove(&scope_id());
}

/// LLM slot only. Parallel lib tests share vault scope `_default`; wiping MCP
/// tickets here races mcp_host / oauth (HTTP 401).
pub fn test_clear_llm() {
    let _guard = io_lock();
    let mut vault = memory_vault();
    vault.set_llm_api_key(None);
    store_memory_vault(&vault);
}

pub fn test_fail_store() {
    let _guard = io_lock();
    memory_fail_map().insert(scope_id(), "unavailable".into());
}

pub fn test_clear_store_fail() {
    let _guard = io_lock();
    memory_fail_map().remove(&scope_id());
}

pub fn test_vault_json() -> serde_json::Value {
    let vault = read_vault().unwrap_or_default();
    serde_json::to_value(vault).unwrap_or_else(|_| serde_json::json!({}))
}

#[test]
fn read_vault_round_trips_devices_without_raw_token() {
    test_clear_scope();
    update_vault(|vault| {
        vault.set_devices(vec![BindDevice {
            device_id: "phone-a".into(),
            device_label: Some("Pixel".into()),
            token_hash: "ee".repeat(32),
            token_hint: Some("••••9f3c".into()),
            revoked: false,
        }]);
    })
    .expect("store");
    let vault = read_vault().expect("vault");
    let phone = vault
        .devices()
        .iter()
        .find(|row| row.device_id == "phone-a")
        .expect("device row");
    assert_eq!(phone.token_hash, "ee".repeat(32));
    let encoded = serde_json::to_string(&vault).expect("encode");
    assert!(!encoded.contains("device_mcp_token"));
    assert!(encoded.contains("token_hash"));
}

#[test]
fn memory_backend_uses_in_memory_keychain_switch() {
    assert!(crate::config::settings::uses_in_memory_keychain());
    let src = concat!(
        include_str!("../../../config/vault/mod.rs"),
        include_str!("../../../config/vault/types.rs"),
        include_str!("../../../config/vault/codec.rs"),
        include_str!("../../../config/vault/store.rs"),
    );
    assert!(src.contains("uses_in_memory_keychain"));
    assert!(src.contains("ACCOUNT_VAULT"));
    assert!(!src.contains("lulu-spark-bind"));
    assert!(!src.contains("lulu-spark-mcp-oauth"));
    assert!(!src.contains("device-tickets.json"));
    assert!(!src.contains("migrate_legacy"));
    assert!(!src.contains("thread_local"));
    assert!(!src.contains("github"));
    assert!(!src.contains("FORCE_STORE"));
}

#[test]
fn memory_store_fail_is_store_error_and_clearing_fail_keeps_data() {
    test_clear_scope();
    update_vault(|vault| vault.set_llm_api_key(Some("sk-keep".into()))).expect("store");
    test_fail_store();
    assert!(matches!(read_vault(), Err(SecretError::Store(_))));
    test_clear_store_fail();
    assert_eq!(
        read_vault().expect("vault after fail off").llm_api_key(),
        Some("sk-keep")
    );
}

#[test]
fn test_clear_scope_wipes_mcp_and_fail_state() {
    test_clear_scope();
    update_vault(|vault| {
        vault.set_llm_api_key(Some("sk".into()));
        vault.set_mcp_slot(
            "spark",
            Some(SlotTicket {
                handle: "ticket".into(),
                state: "live".into(),
                env_suffix: None,
            }),
        );
    })
    .expect("store");
    test_fail_store();
    test_clear_scope();
    let vault = read_vault().expect("empty vault");
    assert!(vault.is_empty());
}

#[test]
fn set_auth_session_round_trips_user_id_and_tokens() {
    test_clear_scope();
    let session = sample_auth_session();
    set_auth_session(&session).expect("store session");
    let got = get_auth_session()
        .expect("read session")
        .expect("session present");
    assert_eq!(got.user.id, "user-42");
    assert_eq!(got.access_token, "access-aaa");
    assert_eq!(got.refresh_token, "refresh-bbb");
}

#[test]
fn delete_auth_session_clears_store_and_omits_key() {
    test_clear_scope();
    set_auth_session(&sample_auth_session()).expect("store session");
    delete_auth_session().expect("delete session");
    assert!(get_auth_session().expect("read after delete").is_none());
    let encoded = serde_json::to_string(&read_vault().expect("vault")).expect("encode");
    assert!(!encoded.contains("supabase-auth"));
}

#[test]
fn auth_session_toggles_is_empty_and_empty_vault_stays_empty() {
    test_clear_scope();
    let empty = read_vault().expect("empty vault");
    assert!(empty.is_empty());
    set_auth_session(&sample_auth_session()).expect("store session");
    assert!(!read_vault().expect("vault with session").is_empty());
    delete_auth_session().expect("delete session");
    assert!(read_vault().expect("vault after delete").is_empty());
}

#[test]
fn keychain_service_and_account_stay_unchanged() {
    assert_eq!(KEYCHAIN_SERVICE, "lulu-spark");
    assert_eq!(ACCOUNT_VAULT, "vault");
}

#[test]
fn auth_session_store_fail_is_store_error() {
    test_clear_scope();
    test_fail_store();
    assert!(matches!(get_auth_session(), Err(SecretError::Store(_))));
    assert!(matches!(
        set_auth_session(&sample_auth_session()),
        Err(SecretError::Store(_))
    ));
    test_clear_store_fail();
}
