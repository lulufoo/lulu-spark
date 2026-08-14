use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings;
use crate::test_support::TestSandbox;

fn with_config<F: FnOnce()>(test: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    test();
}

#[test]
fn host_config_persists_glm_model_and_host_credential_without_echoing_secret() {
    with_config(|| {
        let payload = serde_json::json!({
            "assistant_engine": "host",
            "llm": { "model": "glm-4" },
            "api_key_host": "sk-host"
        });
        secrets::apply_token_payload(&payload).expect("token");
        let mut app_settings = settings::load().expect("load");
        settings::apply_config_payload(&mut app_settings, &payload).expect("config");
        settings::save(&app_settings).expect("save");

        let json = settings::to_config_json(
            &app_settings,
            false,
            false,
            secrets::has_host_key(),
        );
        assert_eq!(json["assistant_engine"], "host");
        assert_eq!(json["llm"]["platform"], "glm");
        assert_eq!(
            json["llm"]["base_url"],
            "https://open.bigmodel.cn/api/paas/v4"
        );
        assert_eq!(json["llm"]["model"], "glm-4");
        assert_eq!(json["has_host_key"], true);
        assert!(json.get("has_cursor_key").is_none());
        assert_eq!(
            secrets::get_secret(KEY_LLM_API_KEY).expect("get"),
            Some("sk-host".into())
        );
        let text = std::fs::read_to_string(settings::config_file_path().expect("path"))
            .expect("toml");
        assert!(!text.contains("sk-host"));
    });
}

#[test]
fn cursor_payload_is_rejected_without_rewriting_existing_settings() {
    with_config(|| {
        let mut app_settings = settings::AppSettings::default();
        let error = settings::apply_config_payload(
            &mut app_settings,
            &serde_json::json!({ "assistant_engine": "cursor" }),
        )
        .expect_err("cursor must be rejected");
        assert!(error.to_string().contains("assistant_engine"));
        assert_eq!(app_settings.assistant_engine, "host");
    });
}

#[test]
fn legacy_cursor_secret_payload_is_ignored() {
    with_config(|| {
        secrets::apply_token_payload(
            &serde_json::json!({ "api_key_cursor": "sk-legacy-cursor" }),
        )
        .expect("token");
        assert!(!secrets::has_host_key());
        assert_eq!(
            secrets::get_secret("llm_api_key_cursor").expect("legacy lookup"),
            None
        );
    });
}
