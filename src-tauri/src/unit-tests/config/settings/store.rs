use std::fs;
use std::path::Path;

use super::{apply_config_payload, load, save, to_config_json};
use crate::config::settings::{
    default_cache_dir, llm_entry_by_type, AppSettings, LlmSettingsEntry,
};
use crate::test_support::TestConfigEnv;

fn prepare_config(env: &TestConfigEnv) -> std::path::PathBuf {
    let path = env.config_file_path();
    fs::create_dir_all(path.parent().expect("config parent")).expect("mkdir");
    path
}

#[test]
fn host_payload_stamps_platform_and_keeps_client_base_url() {
    let mut settings = AppSettings::default();
    apply_config_payload(
        &mut settings,
        &serde_json::json!({
            "assistant_engine": "host",
            "llm": {
                "platform": "client-value",
                "base_url": "https://client.invalid",
                "model": "glm-4"
            }
        }),
    )
    .expect("apply");

    let host = llm_entry_by_type(&settings.llm, "host").expect("host");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://client.invalid");
    assert_eq!(host.model, "glm-4");
}

#[test]
fn empty_base_url_stamps_the_host_default() {
    let mut settings = AppSettings::default();
    apply_config_payload(
        &mut settings,
        &serde_json::json!({
            "assistant_engine": "host",
            "llm": {
                "base_url": "   ",
                "model": "glm-4"
            }
        }),
    )
    .expect("apply");

    let host = llm_entry_by_type(&settings.llm, "host").expect("host");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://open.bigmodel.cn/api/paas/v4");
    assert_eq!(host.model, "glm-4");
}

#[test]
fn openai_payload_stamps_openai_preset_and_keeps_client_base_url() {
    let mut settings = AppSettings::default();
    apply_config_payload(
        &mut settings,
        &serde_json::json!({
            "assistant_engine": "openai",
            "llm": {
                "platform": "client-value",
                "base_url": "https://proxy.openai.test/v1",
                "model": "gpt-4.1"
            }
        }),
    )
    .expect("apply");

    let openai = llm_entry_by_type(&settings.llm, "openai").expect("openai");
    assert_eq!(settings.assistant_engine, "openai");
    assert_eq!(openai.platform, "openai");
    assert_eq!(openai.base_url, "https://proxy.openai.test/v1");
    assert_eq!(openai.model, "gpt-4.1");
}

#[test]
fn model_only_payload_keeps_existing_base_url() {
    let mut settings = AppSettings::default();
    apply_config_payload(
        &mut settings,
        &serde_json::json!({
            "assistant_engine": "host",
            "llm": {
                "base_url": "https://proxy.example/v4",
                "model": "glm-4"
            }
        }),
    )
    .expect("apply custom");
    apply_config_payload(
        &mut settings,
        &serde_json::json!({
            "llm": { "model": "glm-4-air" }
        }),
    )
    .expect("apply model");

    let host = llm_entry_by_type(&settings.llm, "host").expect("host");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://proxy.example/v4");
    assert_eq!(host.model, "glm-4-air");
}

#[test]
fn config_payload_rejects_cursor_and_unknown_engine_values() {
    for value in ["cursor", "anthropic", ""] {
        let mut settings = AppSettings::default();
        assert!(
            apply_config_payload(
                &mut settings,
                &serde_json::json!({ "assistant_engine": value }),
            )
            .is_err(),
            "{value:?} must not be writable"
        );
        assert_eq!(settings.assistant_engine, "host");
    }
}

#[test]
fn legacy_cursor_value_is_preserved_on_load_but_not_resolved_as_an_entry() {
    let dir = tempfile::tempdir().expect("tmp");
    let env = TestConfigEnv::prod(dir.path());
    fs::write(
        prepare_config(&env),
        r#"
assistant_engine = "cursor"

[[llm]]
type = "cursor"
platform = "cursor_agent"
base_url = "(managed by Cursor Agent)"
model = "composer-1"
"#,
    )
    .expect("write");

    let settings = load().expect("load");
    assert_eq!(settings.assistant_engine, "cursor");
    assert_eq!(settings.llm[0].engine_type, "cursor");
    assert!(llm_entry_by_type(&settings.llm, "cursor").is_none());
}

#[test]
fn blank_assistant_engine_is_preserved_on_load() {
    let dir = tempfile::tempdir().expect("tmp");
    let env = TestConfigEnv::prod(dir.path());
    fs::write(prepare_config(&env), "assistant_engine = \"\"\n").expect("write");

    let settings = load().expect("load");
    assert!(settings.assistant_engine.is_empty());
}

#[test]
fn to_config_json_hides_legacy_engine_llm_fields() {
    let mut settings = AppSettings::default();
    settings.assistant_engine = "cursor".into();
    settings.llm.push(LlmSettingsEntry {
        engine_type: "cursor".into(),
        platform: "cursor_agent".into(),
        base_url: "(managed by Cursor Agent)".into(),
        model: "composer-1".into(),
    });

    let json = to_config_json(&settings, true);
    assert_eq!(json["assistant_engine"], "cursor");
    assert_eq!(json["llm"]["model"], "");
    assert_eq!(json["llm"]["platform"], "");
    assert_eq!(json["llm"]["base_url"], "");
    assert!(json.get("has_cursor_key").is_none());
}

#[test]
fn save_roundtrip_keeps_general_settings() {
    let dir = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::prod(dir.path());
    let mut settings = AppSettings::default();
    settings.spark_root = Path::new("/tmp/spark").into();
    settings.github_user_url = "https://github.com/example".into();
    save(&settings).expect("save");

    let loaded = load().expect("load");
    assert_eq!(loaded.spark_root, settings.spark_root);
    assert_eq!(loaded.github_user_url, settings.github_user_url);
}

#[test]
fn spark_github_fields_are_ignored_by_set_config() {
    let mut settings = AppSettings::default();
    let before = settings.github_user_url.clone();
    apply_config_payload(
        &mut settings,
        &serde_json::json!({
            "spark_github_repo_url": "https://github.com/lulufoo/notes",
            "github_user_url": "https://github.com/someone-else"
        }),
    )
    .expect("apply");
    assert_eq!(settings.spark_github_repo_url, "");
    assert_eq!(settings.github_user_url, before);
    let json = to_config_json(&settings, false);
    assert!(json.get("spark_github_repo_url").is_none());
    assert!(json.get("github_user_url").is_none());
    assert!(json.get("has_github_token").is_none());
}

#[test]
fn save_roundtrip_keeps_gateway_port() {
    let dir = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::prod(dir.path());
    let mut settings = AppSettings::default();
    settings.gateway_port = Some(7654);
    save(&settings).expect("save");
    let loaded = load().expect("load");
    assert_eq!(loaded.gateway_port, Some(7654));
    assert_eq!(loaded.effective_gateway_port(), 7654);
}

#[test]
fn to_config_json_includes_gateway_port() {
    let settings = AppSettings::default();
    let json = to_config_json(&settings, false);
    assert_eq!(json["gateway_port"], settings.effective_gateway_port());
}

#[test]
fn to_config_json_exposes_fixed_notes_root() {
    let settings = AppSettings::default();
    let json = to_config_json(&settings, false);
    assert_eq!(
        json["notes_root"],
        default_cache_dir().join("data").join("notes").to_string_lossy().as_ref()
    );
    let mut next = settings;
    apply_config_payload(
        &mut next,
        &serde_json::json!({ "notes_root": "/tmp/should-not-stick" }),
    )
    .expect("apply");
    let after = to_config_json(&next, false);
    assert_eq!(after["notes_root"], json["notes_root"]);
}
