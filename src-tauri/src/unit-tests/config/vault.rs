use super::*;
use crate::test_support::TestSandbox;

pub fn test_clear_scope() {
    let _guard = io_lock();
    let mut vault = memory_vault();
    vault.set_llm_api_key(None);
    vault.set_devices(Vec::new());
    store_memory_vault(&vault);
    let _ = take_legacy_row();
}

pub fn test_clear_llm() {
    let _guard = io_lock();
    let mut vault = memory_vault();
    vault.set_llm_api_key(None);
    store_memory_vault(&vault);
}

pub fn test_seed_legacy_bind_account(account: &str, value: &str) {
    let _guard = io_lock();
    legacy_row(|row| {
        row.bind_accounts
            .insert(account.to_string(), value.to_string());
    });
}

pub fn test_legacy_bind_account(account: &str) -> Option<String> {
    let _guard = io_lock();
    peek_legacy_row().bind_accounts.get(account).cloned()
}

pub fn test_clear_legacy_bind_accounts() {
    let _guard = io_lock();
    delete_leftover_bind_accounts();
}

pub fn test_vault_json() -> serde_json::Value {
    let vault = read_vault().unwrap_or_default();
    serde_json::to_value(vault).unwrap_or_else(|_| serde_json::json!({}))
}

fn seed_legacy_llm(value: &str) {
    let _guard = io_lock();
    legacy_row(|row| row.llm_api_key = Some(value.to_string()));
}

fn seed_legacy_mcp_slot(slot: &str, raw: &str) {
    let _guard = io_lock();
    legacy_row(|row| {
        row.slots.insert(slot.to_string(), raw.to_string());
    });
}

fn seed_legacy_devices(devices: Vec<BindDevice>) {
    let _guard = io_lock();
    legacy_row(|row| row.devices = Some(devices));
}

fn write_device_file(devices: serde_json::Value) {
    let path = crate::config::settings::settings_config_dir()
        .expect("config dir")
        .join("device-tickets.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&serde_json::json!({ "devices": devices })).expect("json"),
    )
    .expect("write device file");
}

fn device_file_exists() -> bool {
    crate::config::settings::settings_config_dir()
        .expect("config dir")
        .join("device-tickets.json")
        .is_file()
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
fn migrate_imports_legacy_items_then_deletes_them() {
    let _sandbox = TestSandbox::new();
    test_clear_scope();
    seed_legacy_llm("sk-legacy");
    seed_legacy_mcp_slot("spark", r#"{"handle":"ab","state":"live"}"#);
    seed_legacy_mcp_slot("cursor_ide", r#"{"handle":"cd","state":"revoked"}"#);
    test_seed_legacy_bind_account("signing", &"aa".repeat(32));
    test_seed_legacy_bind_account("binding", &"bb".repeat(32));
    seed_legacy_devices(vec![BindDevice {
        device_id: "phone-a".into(),
        device_label: Some("Pixel".into()),
        token_hash: "ee".repeat(32),
        token_hint: Some("••••9f3c".into()),
        revoked: false,
    }]);

    migrate_legacy_secrets().expect("migrate");

    let vault = read_vault().expect("vault");
    let phone = vault
        .devices()
        .iter()
        .find(|row| row.device_id == "phone-a")
        .expect("migrate must import the legacy device row");
    assert_eq!(phone.token_hash, "ee".repeat(32));
    assert!(test_legacy_bind_account("signing").is_none());
    assert!(test_legacy_bind_account("binding").is_none());
    let encoded = serde_json::to_string(&vault).expect("encode");
    assert!(!encoded.contains("device_mcp_token"));
    assert!(encoded.contains("token_hash"));
}

#[test]
fn confirm_persisted_rejects_mismatch_and_leaves_legacy() {
    let _sandbox = TestSandbox::new();
    test_clear_scope();
    seed_legacy_llm("sk-keep");

    let mut vault = load_raw().expect("load");
    let legacy = collect_legacy().expect("legacy");
    assert!(merge_legacy(&mut vault, &legacy));
    store_raw(&vault).expect("store merged");
    store_raw(&Vault::default()).expect("corrupt stored tree");
    assert!(confirm_persisted(&vault).is_err());
    assert_eq!(
        peek_legacy_row().llm_api_key.as_deref(),
        Some("sk-keep"),
        "fail-closed must not sweep leftover sources"
    );
}

#[test]
fn device_ledger_file_round_trips_without_migrate_hooks() {
    let _sandbox = TestSandbox::new();
    write_device_file(serde_json::json!([{
        "device_id": "phone-file",
        "device_label": "Pixel",
        "token_hash": "ff".repeat(32),
        "token_hint": "••••9f3c",
        "revoked": false
    }]));
    let rows = read_device_ledger_file()
        .expect("read")
        .expect("file must parse");
    assert_eq!(rows[0].device_id, "phone-file");
    assert_eq!(rows[0].token_hash, "ff".repeat(32));
    delete_device_ledger_file().expect("delete");
    assert!(!device_file_exists(), "device-tickets.json must be deleted");
}

#[test]
fn flat_dev_secrets_toml_is_legacy_not_vault_llm() {
    let parsed = parse_dev_secrets("llm_api_key = \"sk-flat\"\n").expect("parse");
    assert_eq!(parsed.leftover_llm.as_deref(), Some("sk-flat"));
    assert!(
        parsed.vault.is_empty(),
        "flat key must not look like nested llm"
    );
}

#[test]
fn memory_backend_uses_in_memory_keychain_switch() {
    assert!(crate::config::settings::uses_in_memory_keychain());
    let src = include_str!("../../config/vault.rs");
    assert!(src.contains("uses_in_memory_keychain"));
    assert!(src.contains("ACCOUNT_VAULT"));
    assert!(src.contains("LEGACY_KEYCHAIN_SERVICE_BIND"));
    assert!(src.contains("LEGACY_BIND_ACCOUNTS"));
    assert!(!src.contains("thread_local"));
    assert!(!src.contains("THREAD_MIGRATE"));
    assert!(!src.contains("THREAD_FORCE"));
    assert!(!src.contains("github"));
}
