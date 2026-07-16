use crate::config::secrets::{self, KEY_GITHUB_TOKEN, KEY_LLM_API_KEY};
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
        settings::apply_config_payload(&mut s, &payload);
        settings::save(&s).expect("save");
        let v = settings::to_config_json(
            &s,
            secrets::has_github_token(),
            false,
            secrets::has_llm_key(),
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
fn set_config_saves_llm_fields_token_first_without_echoing_key() {
    with_config(|| {
        let payload = serde_json::json!({
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
        settings::apply_config_payload(&mut s, &payload);
        settings::save(&s).expect("save");
        let v = settings::to_config_json(
            &s,
            secrets::has_github_token(),
            secrets::has_meili_key(),
            secrets::has_llm_key(),
        );
        assert_eq!(v["llm"]["platform"], "kimi");
        assert_eq!(v["llm"]["base_url"], "https://api.moonshot.cn");
        assert_eq!(v["llm"]["model"], "moonshot-v1-8k");
        assert_eq!(v["has_llm_key"], true);
        assert!(v.get("api_key").is_none());
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
            "llm": {
                "platform": "glm",
                "base_url": "https://open.bigmodel.cn",
                "model": "glm-4"
            }
        });
        secrets::apply_token_payload(&payload).expect("secrets");
        let mut s = settings::load().expect("load");
        settings::apply_config_payload(&mut s, &payload);
        settings::save(&s).expect("save");
        let v = settings::to_config_json(
            &s,
            secrets::has_github_token(),
            secrets::has_meili_key(),
            secrets::has_llm_key(),
        );
        assert_eq!(v["llm"]["platform"], "glm");
        assert_eq!(v["has_llm_key"], false);
        assert!(v.get("api_key").is_none());
    });
}
