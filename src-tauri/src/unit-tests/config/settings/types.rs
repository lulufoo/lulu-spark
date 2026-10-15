use super::*;

use crate::config::settings::to_config_json;
use crate::test_support::TestConfigEnv;

#[test]
fn defaults_select_host_but_do_not_fabricate_llm_credentials() {
    let _sandbox = crate::test_support::TestSandbox::new();
    let settings = AppSettings::default();
    assert_eq!(settings.assistant_engine, "host");
    assert!(settings.llm.is_empty());

    let json = to_config_json(&settings, &[]).expect("json");
    assert_eq!(json["assistant_engine"], "host");
    assert!(json.get("has_host_key").is_none());
    assert!(json.get("has_cursor_key").is_none());
    for engine in ["host", "openai", "claude", "grok", "kimi", "qwen"] {
        assert_eq!(json["llm"][engine]["key_saved"], false, "{engine}");
        assert!(
            !json["llm"][engine]["base_url"].as_str().unwrap_or("").is_empty(),
            "{engine} falls back to its preset base_url"
        );
    }
}

#[test]
fn gateway_ports_are_written_and_not_reused() {
    assert_eq!(DEFAULT_PROD_GATEWAY_PORT, 7654);
    assert_eq!(DEFAULT_SANDBOX_GATEWAY_PORT, 17654);
    let ports = [
        DEFAULT_PROD_HTTP_PORT,
        DEFAULT_SANDBOX_HTTP_PORT,
        DEFAULT_PROD_MCP_PORT,
        DEFAULT_SANDBOX_MCP_PORT,
        DEFAULT_PROD_GATEWAY_PORT,
        DEFAULT_SANDBOX_GATEWAY_PORT,
    ];
    for (i, left) in ports.iter().enumerate() {
        for right in ports.iter().skip(i + 1) {
            assert_ne!(
                left, right,
                "Gateway ports must not reuse Main Host 8765/18765 or MCP Host 9876/19876"
            );
        }
    }
}

#[test]
fn effective_gateway_port_follows_prod_and_sandbox_planes() {
    let dir = tempfile::tempdir().expect("tmp");
    {
        let _env = TestConfigEnv::prod(dir.path());
        assert_eq!(AppSettings::default().effective_gateway_port().expect("port"), 7654);
    }
    {
        let _env = TestConfigEnv::sandbox(dir.path(), "gwport");
        assert_eq!(AppSettings::default().effective_gateway_port().expect("port"), 17654);
    }
}

#[test]
fn uses_in_memory_keychain_is_true_during_lib_tests() {
    assert!(
        uses_in_memory_keychain().expect("mode"),
        "cargo test --lib must keep Keychain stores in memory"
    );
}

#[test]
fn keychain_memory_switch_is_consulted_by_vault_store() {
    let vault = concat!(
        include_str!("../../../config/vault/mod.rs"),
        include_str!("../../../config/vault/types.rs"),
        include_str!("../../../config/vault/codec.rs"),
        include_str!("../../../config/vault/store.rs"),
    );
    assert!(
        vault.contains("uses_in_memory_keychain"),
        "vault store must consult settings::uses_in_memory_keychain"
    );
    assert!(!include_str!("../../../config/secrets.rs").contains("uses_in_memory_keychain"));
    assert!(!include_str!("../../../services/mcp_oauth.rs").contains("uses_in_memory_keychain"));
}
