use super::*;

use std::fs;
use std::path::Path;

use crate::test_support::TestConfigEnv;

fn prepare_config(env: &TestConfigEnv) -> std::path::PathBuf {
    let path = env.config_file_path();
    fs::create_dir_all(path.parent().expect("config parent")).expect("mkdir");
    path
}

#[test]
fn defaults_select_host_but_do_not_fabricate_llm_credentials() {
    let settings = AppSettings::default();
    assert_eq!(settings.assistant_engine, "host");
    assert!(settings.llm.is_empty());

    let json = to_config_json(&settings, false, false, false);
    assert_eq!(json["assistant_engine"], "host");
    assert_eq!(json["has_host_key"], false);
    assert_eq!(json["has_llm_key"], false);
    assert!(json.get("has_cursor_key").is_none());
}

#[test]
fn host_payload_stamps_the_only_supported_glm_preset() {
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
    assert_eq!(host.base_url, "https://open.bigmodel.cn/api/paas/v4");
    assert_eq!(host.model, "glm-4");
}

#[test]
fn config_payload_rejects_cursor_and_unknown_engine_values() {
    for value in ["cursor", "claude", ""] {
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
fn llm_entry_helpers_only_accept_host() {
    let mut entries = Vec::new();
    upsert_llm_entry(
        &mut entries,
        "host",
        &LlmSettings {
            model: "glm-4".into(),
            ..Default::default()
        },
    )
    .expect("host");
    assert_eq!(llm_entry_by_type(&entries, "host").expect("host").model, "glm-4");
    assert!(upsert_llm_entry(&mut entries, "cursor", &LlmSettings::default()).is_err());
    assert!(llm_entry_by_type(&entries, "cursor").is_none());
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

    let json = to_config_json(&settings, false, false, true);
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
    settings.workbench_knowledge_root = Path::new("/tmp/workbench").into();
    settings.github_user_url = "https://github.com/example".into();
    save(&settings).expect("save");

    let loaded = load().expect("load");
    assert_eq!(loaded.workbench_knowledge_root, settings.workbench_knowledge_root);
    assert_eq!(loaded.github_user_url, settings.github_user_url);
}

#[test]
fn github_remote_helpers_remain_unchanged() {
    assert_eq!(
        github_user_home_from_remote_url("git@github.com:lulufoo/project.git"),
        Some("https://github.com/lulufoo".into())
    );
    assert_eq!(
        workbench_github_blob_base(
            "https://github.com/lulufoo",
            Path::new("/Users/me/Code/lulu-workbench-knowledge"),
        ),
        "https://github.com/lulufoo/lulu-workbench-knowledge/blob/main"
    );
}
