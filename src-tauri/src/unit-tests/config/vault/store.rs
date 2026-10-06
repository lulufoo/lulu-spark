use super::{
    io_lock, memory_fail_map, memory_vault, read_vault, scope_id, store_memory_vault, update_vault,
};
use crate::config::vault::{BindDevice, SecretError, SlotTicket, Vault};

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
            }),
        );
    })
    .expect("store");
    test_fail_store();
    test_clear_scope();
    let vault = read_vault().expect("empty vault");
    assert!(vault.is_empty());
}
