use super::*;

pub fn test_clear_scope() {
    let _guard = io_lock();
    let mut vault = memory_vault();
    vault.set_llm_api_key(None);
    vault.set_devices(Vec::new());
    store_memory_vault(&vault);
}

pub fn test_clear_llm() {
    let _guard = io_lock();
    let mut vault = memory_vault();
    vault.set_llm_api_key(None);
    store_memory_vault(&vault);
}

pub fn test_vault_json() -> serde_json::Value {
    let vault = read_vault().unwrap_or_default();
    serde_json::to_value(vault).unwrap_or_else(|_| serde_json::json!({}))
}

#[test]
fn empty_vault_omits_keys_in_json_and_toml() {
    let vault = Vault::default();
    assert_eq!(vault_to_json(&vault).expect("json"), "{}");
    assert_eq!(vault_to_toml(&vault).expect("toml"), "");
    let json: serde_json::Value =
        serde_json::from_str(&vault_to_json(&vault).expect("parse")).expect("value");
    assert!(json.get("llm").is_none());
    assert!(json.get("mcp-oauth").is_none());
    assert!(json.get("bind").is_none());
    assert!(json.get("github").is_none());
}

#[test]
fn json_and_toml_backends_share_the_same_tree() {
    let mut vault = Vault::default();
    vault.set_llm_api_key(Some("sk-host".into()));
    vault.set_mcp_slot(
        "spark",
        Some(SlotTicket {
            handle: "aa".repeat(32),
            state: "live".into(),
        }),
    );
    vault.set_devices(vec![BindDevice {
        device_id: "phone-a".into(),
        device_label: Some("Pixel".into()),
        token_hash: "bb".repeat(32),
        token_hint: Some("••••9f3c".into()),
        revoked: false,
    }]);

    let from_json = vault_from_json(&vault_to_json(&vault).expect("json")).expect("from json");
    let from_toml = vault_from_toml(&vault_to_toml(&vault).expect("toml")).expect("from toml");
    assert_eq!(from_json, vault);
    assert_eq!(from_toml, vault);
    assert_eq!(from_json, from_toml);
    let toml_text = vault_to_toml(&vault).expect("toml text");
    assert!(toml_text.contains("[llm]"));
    assert!(toml_text.contains("[mcp-oauth.spark]"));
    assert!(toml_text.contains("[[bind.devices]]"));
    assert!(!toml_text.contains("llm_api_key = \"sk-host\"\n[llm]"));
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
fn flat_dev_secrets_toml_is_ignored() {
    let vault = vault_from_toml("llm_api_key = \"sk-flat\"\n").expect("parse");
    assert!(
        vault.is_empty(),
        "flat leftover key must not become vault.llm"
    );
}

#[test]
fn memory_backend_uses_in_memory_keychain_switch() {
    assert!(crate::config::settings::uses_in_memory_keychain());
    let src = include_str!("../../config/vault.rs");
    assert!(src.contains("uses_in_memory_keychain"));
    assert!(src.contains("ACCOUNT_VAULT"));
    assert!(!src.contains("lulu-spark-bind"));
    assert!(!src.contains("lulu-spark-mcp-oauth"));
    assert!(!src.contains("device-tickets.json"));
    assert!(!src.contains("migrate_legacy"));
    assert!(!src.contains("thread_local"));
    assert!(!src.contains("github"));
}
