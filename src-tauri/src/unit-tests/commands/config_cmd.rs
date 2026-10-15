use crate::config::secrets;
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
            "llm_api_key": { "engine": "host", "value": "sk-host" }
        });
        secrets::apply_token_payload(&payload).expect("token");
        let mut app_settings = settings::load().expect("load");
        settings::apply_config_payload(&mut app_settings, &payload).expect("config");
        settings::save(&app_settings).expect("save");

        let json = settings::to_config_json(
            &app_settings,
            &secrets::list_saved_engines(),
        )
        .expect("json");
        assert_eq!(json["assistant_engine"], "host");
        assert_eq!(json["llm"]["host"]["platform"], "glm");
        assert_eq!(
            json["llm"]["host"]["base_url"],
            "https://open.bigmodel.cn/api/paas/v4"
        );
        assert_eq!(json["llm"]["host"]["model"], "glm-4");
        assert_eq!(json["llm"]["host"]["key_saved"], true);
        assert!(json.get("has_host_key").is_none());
        assert_eq!(
            secrets::get_secret("host").expect("get"),
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
fn rejected_engine_payload_does_not_write_host_credential() {
    with_config(|| {
        secrets::set_secret("host", "sk-existing").expect("existing key");
        let result = super::apply_config_payload(&serde_json::json!({
            "assistant_engine": "cursor",
            "llm_api_key": { "engine": "host", "value": "sk-should-not-write" }
        }));

        assert!(result.is_err(), "legacy engine payload must be rejected");
        assert_eq!(
            secrets::get_secret("host").expect("get key"),
            Some("sk-existing".into())
        );
    });
}

#[test]
fn legacy_flat_secret_payload_is_ignored() {
    with_config(|| {
        secrets::apply_token_payload(&serde_json::json!({
            "api_key_host": "sk-legacy-host",
            "api_key_cursor": "sk-legacy-cursor"
        }))
        .expect("token");
        assert!(secrets::list_saved_engines().is_empty());
    });
}

#[test]
fn config_json_reports_key_saved_per_engine_without_leaking_keys() {
    with_config(|| {
        secrets::set_secret("openai", "sk-openai-secret").expect("openai key");
        secrets::set_secret("kimi", "sk-kimi-secret").expect("kimi key");
        let app_settings = settings::AppSettings::default();

        let json = settings::to_config_json(&app_settings, &secrets::list_saved_engines())
            .expect("json");
        for (engine, saved) in [
            ("host", false),
            ("openai", true),
            ("claude", false),
            ("grok", false),
            ("kimi", true),
            ("qwen", false),
        ] {
            assert_eq!(json["llm"][engine]["key_saved"], saved, "{engine}");
        }
        let text = json.to_string();
        assert!(!text.contains("sk-openai-secret"));
        assert!(!text.contains("sk-kimi-secret"));
    });
}

#[test]
fn rejected_key_payload_engine_does_not_write_any_key() {
    with_config(|| {
        let result = super::apply_config_payload(&serde_json::json!({
            "llm_api_key": { "engine": "cursor", "value": "sk-x" }
        }));
        let body = result.expect("secret errors are returned as a json body");
        assert!(body["error"].as_str().unwrap_or("").contains("invalid llm engine"));
        assert!(secrets::list_saved_engines().is_empty());
    });
}
