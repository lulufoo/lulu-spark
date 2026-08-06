use crate::config::secrets::{self, KEY_GITHUB_TOKEN, KEY_LLM_API_KEY, KEY_LLM_API_KEY_CURSOR};
use crate::config::settings;
use crate::test_support::TestSandbox;

fn with_config<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    f();
}

#[test]
fn set_config_payload_updates_settings_and_token_flag() {
    with_config(|| {
        let payload = serde_json::json!({
            "workbench_knowledge_root": "/tmp/my-workbench-knowledge",
            "github_token": "secret-pat"
        });
        secrets::apply_token_payload(&payload).expect("secrets");
        let mut s = settings::load().expect("load");
        settings::apply_config_payload(&mut s, &payload).expect("apply");
        settings::save(&s).expect("save");
        let v = settings::to_config_json(
            &s,
            secrets::has_github_token(),
            false,
            secrets::has_host_key(),
            secrets::has_cursor_key(),
        );
        assert_eq!(v["workbench_knowledge_root"], "/tmp/my-workbench-knowledge");
        assert_eq!(v["has_github_token"], true);
        assert!(v.get("github_token").is_none());
        let s2 = settings::load().expect("reload");
        assert_eq!(
            s2.workbench_knowledge_root.to_string_lossy(),
            "/tmp/my-workbench-knowledge"
        );
        assert_eq!(
            secrets::get_secret(KEY_GITHUB_TOKEN).expect("get"),
            Some("secret-pat".to_string())
        );
    });
}

#[test]
fn set_config_saves_engine_model_and_legacy_api_key_without_echoing() {
    with_config(|| {
        let payload = serde_json::json!({
            "assistant_engine": "host",
            "llm": {
                "platform": "kimi",
                "base_url": "https://api.moonshot.cn",
                "model": "moonshot-v1-8k"
            },
            "api_key": "sk-llm-plain"
        });
        // Order mirrors set_config: token payload, then config payload.
        secrets::apply_token_payload(&payload).expect("secrets");
        let mut s = settings::load().expect("load");
        settings::apply_config_payload(&mut s, &payload).expect("apply");
        settings::save(&s).expect("save");
        let v = settings::to_config_json(
            &s,
            secrets::has_github_token(),
            secrets::has_meili_key(),
            secrets::has_host_key(),
            secrets::has_cursor_key(),
        );
        assert_eq!(v["assistant_engine"], "host");
        // Client platform/base_url ignored; host builtin preset stamped for Host consumption.
        assert_eq!(v["llm"]["platform"], "glm");
        assert_eq!(v["llm"]["base_url"], "https://open.bigmodel.cn/api/paas/v4");
        assert_eq!(v["llm"]["model"], "moonshot-v1-8k");
        assert_eq!(v["has_llm_key"], true);
        assert_eq!(v["has_host_key"], true);
        assert_eq!(v["has_cursor_key"], false);
        assert!(v.get("api_key").is_none());
        assert!(v.get("api_key_host").is_none());
        assert!(v["llm"].get("api_key").is_none());
        assert_eq!(
            secrets::get_secret(KEY_LLM_API_KEY).expect("get"),
            Some("sk-llm-plain".to_string())
        );
        let toml_text = std::fs::read_to_string(settings::config_file_path()).expect("toml");
        assert!(!toml_text.contains("sk-llm-plain"));
        assert!(!toml_text.contains("api_key"));
    });
}

#[test]
fn set_config_llm_without_api_key_keeps_has_llm_key_false() {
    with_config(|| {
        let payload = serde_json::json!({
            "assistant_engine": "host",
            "llm": {
                "model": "glm-4"
            }
        });
        secrets::apply_token_payload(&payload).expect("secrets");
        let mut s = settings::load().expect("load");
        settings::apply_config_payload(&mut s, &payload).expect("apply");
        settings::save(&s).expect("save");
        let v = settings::to_config_json(
            &s,
            secrets::has_github_token(),
            secrets::has_meili_key(),
            secrets::has_host_key(),
            secrets::has_cursor_key(),
        );
        assert_eq!(v["llm"]["model"], "glm-4");
        assert_eq!(v["llm"]["platform"], "glm");
        assert_eq!(v["llm"]["base_url"], "https://open.bigmodel.cn/api/paas/v4");
        assert_eq!(v["has_llm_key"], false);
        assert_eq!(v["has_host_key"], false);
        assert!(v.get("api_key").is_none());
    });
}

#[test]
fn set_config_per_category_keys_independent_and_blank_omission_keeps_secrets() {
    with_config(|| {
        let host_payload = serde_json::json!({
            "assistant_engine": "host",
            "llm": { "model": "glm-4" },
            "api_key_host": "sk-host"
        });
        secrets::apply_token_payload(&host_payload).expect("secrets host");
        let mut s = settings::load().expect("load");
        settings::apply_config_payload(&mut s, &host_payload).expect("apply host");
        settings::save(&s).expect("save");

        let cursor_payload = serde_json::json!({
            "assistant_engine": "cursor",
            "llm": { "model": "composer-1" },
            "api_key_cursor": "sk-cursor"
        });
        secrets::apply_token_payload(&cursor_payload).expect("secrets cursor");
        settings::apply_config_payload(&mut s, &cursor_payload).expect("apply cursor");
        settings::save(&s).expect("save");

        assert_eq!(
            secrets::get_secret(KEY_LLM_API_KEY).expect("get"),
            Some("sk-host".to_string())
        );
        assert_eq!(
            secrets::get_secret(KEY_LLM_API_KEY_CURSOR).expect("get"),
            Some("sk-cursor".to_string())
        );

        // Blank credential omitted — model-only save must not clear keys.
        let no_key = serde_json::json!({
            "assistant_engine": "host",
            "llm": { "model": "glm-4-air" }
        });
        secrets::apply_token_payload(&no_key).expect("secrets none");
        settings::apply_config_payload(&mut s, &no_key).expect("apply none");
        settings::save(&s).expect("save");
        assert_eq!(
            secrets::get_secret(KEY_LLM_API_KEY).expect("get"),
            Some("sk-host".to_string())
        );
        assert_eq!(
            secrets::get_secret(KEY_LLM_API_KEY_CURSOR).expect("get"),
            Some("sk-cursor".to_string())
        );

        let v = settings::to_config_json(
            &s,
            secrets::has_github_token(),
            secrets::has_meili_key(),
            secrets::has_host_key(),
            secrets::has_cursor_key(),
        );
        assert_eq!(v["assistant_engine"], "host");
        assert_eq!(v["llm"]["model"], "glm-4-air");
        assert_eq!(v["has_host_key"], true);
        assert_eq!(v["has_cursor_key"], true);
        assert_eq!(v["has_llm_key"], true);
        assert!(v.get("api_key_host").is_none());
        assert!(v.get("api_key_cursor").is_none());
    });
}

#[test]
fn set_config_rejects_illegal_assistant_engine_without_persisting() {
    with_config(|| {
        let ok = serde_json::json!({
            "assistant_engine": "host",
            "llm": { "model": "m1" }
        });
        secrets::apply_token_payload(&ok).expect("secrets");
        let mut s = settings::load().expect("load");
        settings::apply_config_payload(&mut s, &ok).expect("apply ok");
        settings::save(&s).expect("save");

        let bad = serde_json::json!({ "assistant_engine": "claude" });
        let err = settings::apply_config_payload(&mut s, &bad).expect_err("reject");
        assert!(err.to_string().contains("assistant_engine") || err.to_string().contains("claude"));
        assert_eq!(s.assistant_engine, "host");
        let reloaded = settings::load().expect("reload");
        assert_eq!(reloaded.assistant_engine, "host");
    });
}
