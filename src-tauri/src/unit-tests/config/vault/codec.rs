use super::{vault_from_json, vault_from_toml, vault_to_json, vault_to_toml};
use crate::config::vault::{AuthSession, AuthUser, BindDevice, SlotTicket, Vault};

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
fn flat_dev_secrets_toml_is_ignored() {
    let vault = vault_from_toml("llm_api_key = \"sk-flat\"\n").expect("parse");
    assert!(
        vault.is_empty(),
        "flat leftover key must not become vault.llm"
    );
}

#[test]
fn empty_vault_omits_supabase_auth_and_is_empty_without_session() {
    let vault = Vault::default();
    assert!(vault.is_empty());
    assert!(vault.auth_session().is_none());
    assert_eq!(vault_to_json(&vault).expect("json"), "{}");
    assert_eq!(vault_to_toml(&vault).expect("toml"), "");
    let json: serde_json::Value =
        serde_json::from_str(&vault_to_json(&vault).expect("parse")).expect("value");
    assert!(json.get("supabase-auth").is_none());
}

#[test]
fn auth_session_encodes_as_supabase_auth_without_provider_token() {
    let mut vault = Vault::default();
    vault.set_auth_session(Some(sample_auth_session()));
    assert!(!vault.is_empty());
    assert_eq!(
        vault.auth_session().map(|s| s.user.id.as_str()),
        Some("user-42")
    );

    let json_text = vault_to_json(&vault).expect("json");
    let toml_text = vault_to_toml(&vault).expect("toml");
    assert!(json_text.contains("supabase-auth"));
    assert!(toml_text.contains("supabase-auth") || toml_text.contains("[supabase-auth]"));
    assert!(!json_text.contains("provider_token"));
    assert!(!toml_text.contains("provider_token"));

    let json: serde_json::Value = serde_json::from_str(&json_text).expect("value");
    assert!(json.get("supabase-auth").is_some());
    assert!(json.get("provider_token").is_none());

    let from_json = vault_from_json(&json_text).expect("from json");
    let from_toml = vault_from_toml(&toml_text).expect("from toml");
    assert_eq!(from_json, vault);
    assert_eq!(from_toml, vault);

    vault.set_auth_session(None);
    assert!(vault.is_empty());
    assert_eq!(vault_to_json(&vault).expect("cleared json"), "{}");
    assert_eq!(vault_to_toml(&vault).expect("cleared toml"), "");
    let cleared: serde_json::Value =
        serde_json::from_str(&vault_to_json(&vault).expect("cleared parse")).expect("value");
    assert!(cleared.get("supabase-auth").is_none());
}
