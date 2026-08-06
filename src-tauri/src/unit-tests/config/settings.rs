use super::*;
use std::fs;
use std::path::Path;

struct IsolatedConfigGuard {
    prev: Option<String>,
}

impl IsolatedConfigGuard {
    fn set(dir: &Path) -> Self {
        let prev = std::env::var("LULU_WB_CONFIG_DIR").ok();
        unsafe { std::env::set_var("LULU_WB_CONFIG_DIR", dir) };
        crate::config::secrets::test_secrets_clear();
        Self { prev }
    }
}

impl Drop for IsolatedConfigGuard {
    fn drop(&mut self) {
        match self.prev.take() {
            Some(v) => unsafe { std::env::set_var("LULU_WB_CONFIG_DIR", v) },
            None => unsafe { std::env::remove_var("LULU_WB_CONFIG_DIR") },
        }
        crate::config::secrets::test_secrets_clear();
    }
}

#[test]
fn load_reads_config_toml_fields() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    let wb = dir.path().join("my-workbench-knowledge");
    let kc = dir.path().join("my-knowledge-corpus");
    let cache = dir.path().join("my-cache");
    fs::write(
        dir.path().join(PROD_CONFIG_FILE_NAME),
        format!(
            r#"
workbench_knowledge_root = "{}"
knowledge_corpus_root = "{}"
cache_dir = "{}"
meili_url = "http://127.0.0.1:7701"
github_user_url = "https://github.com/example"
"#,
            wb.display(),
            kc.display(),
            cache.display()
        ),
    )
    .expect("write");
    let s = load().expect("load");
    assert_eq!(s.workbench_knowledge_root, wb);
    assert_eq!(s.knowledge_corpus_root, kc);
    assert_eq!(s.cache_dir, cache);
    assert_eq!(s.meili_url, "http://127.0.0.1:7701");
    assert_eq!(s.github_user_url, "https://github.com/example");
}

#[test]
fn github_user_home_from_https_remote() {
    assert_eq!(
        github_user_home_from_remote_url(
            "https://github.com/lulufoo/lulu-workbench-knowledge.git"
        ),
        Some("https://github.com/lulufoo".to_string())
    );
}

#[test]
fn github_user_home_from_ssh_remote() {
    assert_eq!(
        github_user_home_from_remote_url("git@github.com:lulufoo/lulu-workbench-knowledge.git"),
        Some("https://github.com/lulufoo".to_string())
    );
}

#[test]
fn workbench_github_blob_base_from_user_url_and_root() {
    let root = PathBuf::from("/Users/me/Code/lulu-workbench-knowledge");
    assert_eq!(
        workbench_github_blob_base("https://github.com/lulufoo", &root),
        "https://github.com/lulufoo/lulu-workbench-knowledge/blob/main"
    );
    assert_eq!(workbench_github_blob_base("", &root), "");
}

#[test]
fn load_defaults_when_no_config_file() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    let s = load().expect("load");
    assert_eq!(s.cache_dir, default_cache_dir());
    assert_eq!(s.meili_url, "http://localhost:7700");
}

#[test]
fn save_roundtrip_updates_file() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    let mut s = AppSettings::default();
    s.workbench_knowledge_root = dir.path().join("workbench-x");
    save(&s).expect("save");
    let s2 = load().expect("reload");
    assert_eq!(s2.workbench_knowledge_root, s.workbench_knowledge_root);
}

#[test]
fn to_config_json_includes_flags_without_token() {
    let s = AppSettings::default();
    let v = to_config_json(&s, true, false, false, false);
    assert_eq!(v["has_github_token"], true);
    assert_eq!(v["has_meili_key"], false);
    assert_eq!(v["has_llm_key"], false);
    assert_eq!(v["has_host_key"], false);
    assert_eq!(v["has_cursor_key"], false);
    assert!(v.get("github_token").is_none());
    assert!(v.get("meili_master_key").is_none());
    assert!(v.get("api_key").is_none());
    assert!(v.get("api_key_host").is_none());
    assert!(v.get("api_key_cursor").is_none());
}

#[test]
fn to_config_json_includes_llm_fields_without_plaintext_key() {
    let mut s = AppSettings::default();
    upsert_llm_entry(
        &mut s.llm,
        "host",
        &LlmSettings {
            platform: "kimi".into(),
            base_url: "https://api.moonshot.cn".into(),
            model: "moonshot-v1-8k".into(),
        },
    )
    .expect("upsert host");
    let v = to_config_json(&s, false, false, true, false);
    assert_eq!(v["llm"]["platform"], "kimi");
    assert_eq!(v["llm"]["base_url"], "https://api.moonshot.cn");
    assert_eq!(v["llm"]["model"], "moonshot-v1-8k");
    assert_eq!(v["has_llm_key"], true);
    assert_eq!(v["has_host_key"], true);
    assert_eq!(v["has_cursor_key"], false);
    assert!(v.get("api_key").is_none());
    assert!(v.get("api_key_host").is_none());
    assert!(v.get("api_key_cursor").is_none());
    assert!(v["llm"].get("api_key").is_none());
}

#[test]
fn apply_config_payload_updates_engine_model_and_stamps_readonly_preset() {
    let mut s = AppSettings::default();
    apply_config_payload(
        &mut s,
        &serde_json::json!({
            "assistant_engine": "host",
            "llm": {
                "platform": "kimi",
                "base_url": "https://api.moonshot.cn",
                "model": "glm-4"
            },
            "api_key": "should-not-land-in-settings"
        }),
    )
    .expect("apply");
    // Client platform/base_url ignored; builtin host preset stamped; model kept.
    assert_eq!(s.assistant_engine, "host");
    let host = llm_entry_by_type(&s.llm, "host").expect("host entry");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://open.bigmodel.cn/api/paas/v4");
    assert_eq!(host.model, "glm-4");
    let text = toml::to_string(&s).expect("serialize");
    assert!(!text.contains("should-not-land-in-settings"));
    assert!(!text.contains("api_key"));
}

#[test]
fn apply_config_payload_ignores_illegal_llm_types_without_clobber() {
    let mut s = AppSettings::default();
    upsert_llm_entry(
        &mut s.llm,
        "host",
        &LlmSettings {
            platform: "openai_compatible".into(),
            base_url: "https://example.com".into(),
            model: "gpt-4o".into(),
        },
    )
    .expect("seed host");
    apply_config_payload(
        &mut s,
        &serde_json::json!({
            "llm": {
                "platform": 123,
                "base_url": true,
                "model": {"x": 1}
            }
        }),
    )
    .expect("apply");
    // No assistant_engine → no stamp; invalid types ignored; prior metadata kept.
    let host = llm_entry_by_type(&s.llm, "host").expect("host entry");
    assert_eq!(host.platform, "openai_compatible");
    assert_eq!(host.base_url, "https://example.com");
    assert_eq!(host.model, "gpt-4o");
}

#[test]
fn llm_fields_roundtrip_in_config_toml_without_api_key() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    let mut s = AppSettings::default();
    upsert_llm_entry(
        &mut s.llm,
        "host",
        &LlmSettings {
            platform: "openai_compatible".into(),
            base_url: "https://api.openai.com".into(),
            model: "gpt-4o-mini".into(),
        },
    )
    .expect("upsert host");
    save(&s).expect("save");
    let text = fs::read_to_string(config_file_path()).expect("read");
    assert!(text.contains("[[llm]]") || text.contains("[llm]"));
    assert!(text.contains("type") || text.contains("host"));
    assert!(!text.contains("api_key"));
    let s2 = load().expect("reload");
    let host = llm_entry_by_type(&s2.llm, "host").expect("host entry");
    assert_eq!(host.platform, "openai_compatible");
    assert_eq!(host.base_url, "https://api.openai.com");
    assert_eq!(host.model, "gpt-4o-mini");
}

#[test]
fn is_unstable_cache_dir_detects_macos_tempfile() {
    assert!(is_unstable_cache_dir(Path::new(
        "/var/folders/xx/T/.tmp7vvARz/cache"
    )));
    assert!(!is_unstable_cache_dir(Path::new(
        "/Users/me/.cache/lulu-workbench"
    )));
}

#[test]
fn normalize_cache_dir_resets_ephemeral_path() {
    let mut s = AppSettings::default();
    s.cache_dir = PathBuf::from("/var/folders/xx/T/.tmp7vvARz/cache");
    normalize_cache_dir(&mut s);
    assert_eq!(s.cache_dir, default_cache_dir());
}

#[test]
fn apply_config_payload_ignores_cache_dir() {
    let mut s = AppSettings::default();
    let before = s.cache_dir.clone();
    apply_config_payload(
        &mut s,
        &serde_json::json!({ "cache_dir": "/var/folders/x/T/.tmp7vvARz/cache" }),
    )
    .expect("apply");
    assert_eq!(s.cache_dir, before);
}

#[test]
fn migrate_llm_to_engine_locks_host_when_no_existing_engine() {
    let legacy = LlmSettings {
        platform: "kimi".into(),
        base_url: "https://api.moonshot.cn".into(),
        model: "moonshot-v1-8k".into(),
    };
    let slice = migrate_llm_to_engine(&legacy, Some("sk-legacy"), None);
    assert_eq!(slice.assistant_engine, "host");
    assert_eq!(slice.model, "moonshot-v1-8k");
    assert_eq!(slice.platform, "kimi");
    assert_eq!(slice.base_url, "https://api.moonshot.cn");
    assert_eq!(slice.host_api_key.as_deref(), Some("sk-legacy"));
}

#[test]
fn migrate_llm_to_engine_respects_existing_cursor_engine() {
    let legacy = LlmSettings {
        platform: "glm".into(),
        base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
        model: "glm-4".into(),
    };
    let slice = migrate_llm_to_engine(&legacy, Some("sk-host"), Some("cursor"));
    assert_eq!(slice.assistant_engine, "cursor");
    assert_eq!(slice.model, "glm-4");
    assert_eq!(slice.host_api_key.as_deref(), Some("sk-host"));
}

#[test]
fn migrate_llm_to_engine_treats_blank_existing_as_host() {
    let legacy = LlmSettings {
        platform: "x".into(),
        base_url: "https://x".into(),
        model: "m".into(),
    };
    let slice = migrate_llm_to_engine(&legacy, None, Some("   "));
    assert_eq!(slice.assistant_engine, "host");
    assert!(slice.host_api_key.is_none());
}

#[test]
fn apply_config_payload_rejects_illegal_assistant_engine() {
    let mut s = AppSettings::default();
    s.assistant_engine = "host".into();
    let err = apply_config_payload(
        &mut s,
        &serde_json::json!({ "assistant_engine": "claude" }),
    )
    .expect_err("illegal engine");
    assert!(
        err.to_string().contains("assistant_engine") || err.to_string().contains("claude"),
        "err={err}"
    );
    assert_eq!(s.assistant_engine, "host");
}

#[test]
fn apply_config_payload_accepts_host_and_cursor_engines() {
    let mut s = AppSettings::default();
    apply_config_payload(&mut s, &serde_json::json!({ "assistant_engine": "cursor" }))
        .expect("cursor");
    assert_eq!(s.assistant_engine, "cursor");
    let cursor = llm_entry_by_type(&s.llm, "cursor").expect("cursor entry");
    assert_eq!(cursor.platform, "cursor_agent");
    assert_eq!(cursor.base_url, "(managed by Cursor Agent)");
    apply_config_payload(&mut s, &serde_json::json!({ "assistant_engine": "host" }))
        .expect("host");
    assert_eq!(s.assistant_engine, "host");
    let host = llm_entry_by_type(&s.llm, "host").expect("host entry");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://open.bigmodel.cn/api/paas/v4");
}

#[test]
fn load_stamps_builtin_preset_when_entry_platform_base_url_blank() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    fs::write(
        dir.path().join(PROD_CONFIG_FILE_NAME),
        r#"
assistant_engine = "host"

[[llm]]
type = "host"
model = "glm-4"
"#,
    )
    .expect("write");
    let s = load().expect("load");
    assert_eq!(s.assistant_engine, "host");
    let host = llm_entry_by_type(&s.llm, "host").expect("host entry");
    assert_eq!(host.model, "glm-4");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://open.bigmodel.cn/api/paas/v4");
}

#[test]
fn load_typed_llm_list_host_entry_without_requiring_cursor() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    fs::write(
        dir.path().join(PROD_CONFIG_FILE_NAME),
        r#"
assistant_engine = "host"

[[llm]]
type = "host"
platform = "kimi"
base_url = "https://api.moonshot.cn"
model = "moonshot-v1-8k"
"#,
    )
    .expect("write");
    let s = load().expect("load");
    assert_eq!(s.assistant_engine, "host");
    let host = llm_entry_by_type(&s.llm, "host").expect("host entry");
    assert_eq!(host.model, "moonshot-v1-8k");
    assert_eq!(host.platform, "kimi");
    assert_eq!(host.base_url, "https://api.moonshot.cn");
    assert!(llm_entry_by_type(&s.llm, "cursor").is_none());
}

#[test]
fn load_respects_existing_cursor_assistant_engine_while_keeping_llm_model() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    fs::write(
        dir.path().join(PROD_CONFIG_FILE_NAME),
        r#"
assistant_engine = "cursor"

[[llm]]
type = "cursor"
platform = "glm"
base_url = "https://open.bigmodel.cn/api/paas/v4"
model = "composer-1"
"#,
    )
    .expect("write");
    let s = load().expect("load");
    assert_eq!(s.assistant_engine, "cursor");
    let cursor = llm_entry_by_type(&s.llm, "cursor").expect("cursor entry");
    assert_eq!(cursor.model, "composer-1");
}

struct TestModeGuard(Option<String>);

impl TestModeGuard {
    fn set(value: Option<&str>) -> Self {
        let prev = std::env::var("TEST_MODE").ok();
        match value {
            Some(v) => unsafe { std::env::set_var("TEST_MODE", v) },
            None => unsafe { std::env::remove_var("TEST_MODE") },
        }
        TestModeGuard(prev)
    }
}

impl Drop for TestModeGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(v) => unsafe { std::env::set_var("TEST_MODE", v) },
            None => unsafe { std::env::remove_var("TEST_MODE") },
        }
    }
}

#[test]
fn is_test_mode_false_when_unset() {
    let _g = TestModeGuard::set(None);
    assert!(!is_test_mode());
    assert_eq!(test_mode_kind(), TestModeKind::Prod);
}

#[test]
fn is_test_mode_true_when_one() {
    let _g = TestModeGuard::set(Some("1"));
    assert!(is_test_mode());
    assert_eq!(test_mode_kind(), TestModeKind::TestSandbox);
}

#[test]
fn is_test_mode_false_for_non_one_values() {
    for v in ["0", "", "true", "2"] {
        let _g = TestModeGuard::set(Some(v));
        assert!(!is_test_mode(), "TEST_MODE={v}");
        assert_eq!(test_mode_kind(), TestModeKind::Prod, "TEST_MODE={v}");
    }
}

#[test]
fn test_mode_save_writes_dev_config_not_prod() {
    crate::test_support::with_config_test_serial(|| {
        let _tm = TestModeGuard::set(Some("1"));
        let dir = tempfile::tempdir().expect("tmp");
        let wb = dir.path().join("wb");
        fs::create_dir_all(&wb).expect("mkdir wb");
        let mut s = AppSettings::default();
        s.workbench_knowledge_root = wb;
        save(&s).expect("save");
        let dev_path = dev_config_file_path();
        assert!(dev_path.is_file());
        let text = fs::read_to_string(&dev_path).expect("read dev config");
        assert!(text.contains("workbench_knowledge_root"));
        assert!(!text.contains("test_mode"));
        assert!(!text.contains("TEST_MODE"));
    });
}

#[test]
fn save_rejects_writing_prod_config_in_test_mode() {
    crate::test_support::with_config_test_serial(|| {
        let _tm = TestModeGuard::set(Some("1"));
        let prod = prod_config_file_path();
        let before = prod.is_file().then(|| fs::read_to_string(&prod).ok()).flatten();
        let mut s = AppSettings::default();
        s.workbench_knowledge_root = PathBuf::from("/tmp/should-not-persist");
        let err = save(&s);
        assert!(err.is_ok());
        if let Some(prev) = before {
            assert_eq!(fs::read_to_string(&prod).ok(), Some(prev));
        } else {
            assert!(!prod.is_file());
        }
    });
}

#[test]
fn llm_entry_by_type_returns_none_for_empty_list_or_missing_type() {
    let empty: Vec<LlmSettingsEntry> = vec![];
    assert!(llm_entry_by_type(&empty, "host").is_none());
    assert!(llm_entry_by_type(&empty, "cursor").is_none());

    let mut only_host = vec![];
    upsert_llm_entry(
        &mut only_host,
        "host",
        &LlmSettings {
            platform: "glm".into(),
            base_url: "https://host.example".into(),
            model: "host-model".into(),
        },
    )
    .expect("upsert host");
    assert!(llm_entry_by_type(&only_host, "cursor").is_none());
    let host = llm_entry_by_type(&only_host, "host").expect("host");
    assert_eq!(host.engine_type, "host");
    assert_eq!(host.model, "host-model");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://host.example");
}

#[test]
fn upsert_llm_entry_updates_existing_and_creates_missing_type() {
    let mut entries = vec![];
    upsert_llm_entry(
        &mut entries,
        "host",
        &LlmSettings {
            platform: "glm".into(),
            base_url: "https://host.example".into(),
            model: "m1".into(),
        },
    )
    .expect("create host");
    assert_eq!(entries.len(), 1);

    upsert_llm_entry(
        &mut entries,
        "host",
        &LlmSettings {
            platform: "glm".into(),
            base_url: "https://host.example".into(),
            model: "m2".into(),
        },
    )
    .expect("update host");
    assert_eq!(entries.len(), 1);
    assert_eq!(llm_entry_by_type(&entries, "host").unwrap().model, "m2");

    upsert_llm_entry(
        &mut entries,
        "cursor",
        &LlmSettings {
            platform: "cursor_agent".into(),
            base_url: "(managed by Cursor Agent)".into(),
            model: "composer-1".into(),
        },
    )
    .expect("create cursor");
    assert_eq!(entries.len(), 2);
    assert_eq!(
        llm_entry_by_type(&entries, "cursor").unwrap().model,
        "composer-1"
    );
    assert_eq!(llm_entry_by_type(&entries, "host").unwrap().model, "m2");
}

#[test]
fn upsert_llm_entry_rejects_illegal_engine_type_without_writing() {
    let mut entries = vec![];
    let err = upsert_llm_entry(
        &mut entries,
        "claude",
        &LlmSettings {
            model: "should-not-land".into(),
            ..Default::default()
        },
    )
    .expect_err("illegal type");
    assert!(
        err.to_string().contains("type") || err.to_string().contains("claude"),
        "err={err}"
    );
    assert!(entries.is_empty());
}

#[test]
fn app_settings_llm_is_typed_list_not_single_slot() {
    let mut s = AppSettings::default();
    assert!(s.llm.is_empty());
    upsert_llm_entry(
        &mut s.llm,
        "cursor",
        &LlmSettings {
            model: "composer-1".into(),
            ..Default::default()
        },
    )
    .expect("cursor");
    upsert_llm_entry(
        &mut s.llm,
        "host",
        &LlmSettings {
            model: "glm-4".into(),
            ..Default::default()
        },
    )
    .expect("host");
    assert_eq!(s.llm.len(), 2);
    assert_eq!(llm_entry_by_type(&s.llm, "host").unwrap().model, "glm-4");
    assert_eq!(
        llm_entry_by_type(&s.llm, "cursor").unwrap().model,
        "composer-1"
    );
}

// --- T2: flat facade ↔ typed list (save isolation / readonly preset) ---

#[test]
fn to_config_json_exposes_flat_current_llm_not_list() {
    let mut s = AppSettings::default();
    s.assistant_engine = "cursor".into();
    upsert_llm_entry(
        &mut s.llm,
        "host",
        &LlmSettings {
            platform: "glm".into(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            model: "host-model".into(),
        },
    )
    .expect("host");
    upsert_llm_entry(
        &mut s.llm,
        "cursor",
        &LlmSettings {
            platform: "cursor_agent".into(),
            base_url: "(managed by Cursor Agent)".into(),
            model: "cursor-model".into(),
        },
    )
    .expect("cursor");
    let v = to_config_json(&s, false, false, false, false);
    assert_eq!(v["assistant_engine"], "cursor");
    assert!(v["llm"].is_object(), "facade llm must be flat object, not list");
    assert!(v["llm"].as_array().is_none());
    assert_eq!(v["llm"]["model"], "cursor-model");
    assert_eq!(v["llm"]["platform"], "cursor_agent");
    assert_eq!(v["llm"]["base_url"], "(managed by Cursor Agent)");
}

#[test]
fn apply_config_payload_first_save_creates_current_type_with_preset_stamp() {
    let mut s = AppSettings::default();
    assert!(s.llm.is_empty());
    apply_config_payload(
        &mut s,
        &serde_json::json!({
            "llm": {
                "platform": "kimi",
                "base_url": "https://api.moonshot.cn",
                "model": "first-host-model"
            }
        }),
    )
    .expect("apply");
    assert_eq!(s.llm.len(), 1);
    let host = llm_entry_by_type(&s.llm, "host").expect("host created");
    assert_eq!(host.model, "first-host-model");
    // Client preset fields ignored; builtin host preset stamped onto the new entry.
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://open.bigmodel.cn/api/paas/v4");
    assert!(llm_entry_by_type(&s.llm, "cursor").is_none());
}

#[test]
fn apply_config_payload_save_current_type_preserves_other_type() {
    let mut s = AppSettings::default();
    s.assistant_engine = "host".into();
    upsert_llm_entry(
        &mut s.llm,
        "host",
        &LlmSettings {
            platform: "glm".into(),
            base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            model: "host-before".into(),
        },
    )
    .expect("host");
    upsert_llm_entry(
        &mut s.llm,
        "cursor",
        &LlmSettings {
            platform: "cursor_agent".into(),
            base_url: "(managed by Cursor Agent)".into(),
            model: "cursor-keep".into(),
        },
    )
    .expect("cursor");
    apply_config_payload(
        &mut s,
        &serde_json::json!({
            "llm": {
                "platform": "should-not-apply",
                "base_url": "https://evil.example",
                "model": "host-after"
            }
        }),
    )
    .expect("apply");
    let host = llm_entry_by_type(&s.llm, "host").expect("host");
    assert_eq!(host.model, "host-after");
    assert_eq!(host.platform, "glm");
    assert_eq!(host.base_url, "https://open.bigmodel.cn/api/paas/v4");
    let cursor = llm_entry_by_type(&s.llm, "cursor").expect("cursor preserved");
    assert_eq!(cursor.model, "cursor-keep");
    assert_eq!(cursor.platform, "cursor_agent");
    assert_eq!(cursor.base_url, "(managed by Cursor Agent)");
}

#[test]
fn load_apply_to_json_roundtrip_does_not_erase_inactive_type() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = IsolatedConfigGuard::set(dir.path());
    fs::write(
        dir.path().join(PROD_CONFIG_FILE_NAME),
        r#"
assistant_engine = "host"

[[llm]]
type = "host"
platform = "glm"
base_url = "https://open.bigmodel.cn/api/paas/v4"
model = "host-a"

[[llm]]
type = "cursor"
platform = "cursor_agent"
base_url = "(managed by Cursor Agent)"
model = "cursor-b"
"#,
    )
    .expect("write");
    let mut s = load().expect("load");
    apply_config_payload(
        &mut s,
        &serde_json::json!({ "llm": { "model": "host-c" } }),
    )
    .expect("apply");
    let flat = to_config_json(&s, false, false, false, false);
    assert_eq!(flat["assistant_engine"], "host");
    assert_eq!(flat["llm"]["model"], "host-c");
    assert!(flat["llm"].is_object());
    save(&s).expect("save");
    let s2 = load().expect("reload");
    assert_eq!(
        llm_entry_by_type(&s2.llm, "host").expect("host").model,
        "host-c"
    );
    let cursor = llm_entry_by_type(&s2.llm, "cursor").expect("cursor intact");
    assert_eq!(cursor.model, "cursor-b");
    assert_eq!(cursor.platform, "cursor_agent");
    assert_eq!(cursor.base_url, "(managed by Cursor Agent)");
}
