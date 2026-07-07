use super::*;
use std::fs;
use std::path::Path;

struct EnvGuard;

impl EnvGuard {
    fn with_config_dir(dir: &Path) -> Self {
        set_test_config_dir(Some(dir.to_path_buf()));
        crate::config::secrets::test_secrets_clear();
        Self
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        set_test_config_dir(None);
        crate::config::secrets::test_secrets_clear();
    }
}

#[test]
fn load_reads_config_toml_fields() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = EnvGuard::with_config_dir(dir.path());
    let wb = dir.path().join("my-workbench-knowledge");
    let kc = dir.path().join("my-knowledge-corpus");
    let cache = dir.path().join("my-cache");
    fs::write(
        dir.path().join("config.toml"),
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
    let _guard = EnvGuard::with_config_dir(dir.path());
    let s = load().expect("load");
    assert_eq!(s.cache_dir, default_cache_dir());
    assert_eq!(s.meili_url, "http://localhost:7700");
}

#[test]
fn save_roundtrip_updates_file() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = EnvGuard::with_config_dir(dir.path());
    let mut s = AppSettings::default();
    s.workbench_knowledge_root = dir.path().join("workbench-x");
    save(&s).expect("save");
    let s2 = load().expect("reload");
    assert_eq!(s2.workbench_knowledge_root, s.workbench_knowledge_root);
}

#[test]
fn to_config_json_includes_flags_without_token() {
    let s = AppSettings::default();
    let v = to_config_json(&s, true, false);
    assert_eq!(v["has_github_token"], true);
    assert_eq!(v["has_meili_key"], false);
    assert!(v.get("github_token").is_none());
    assert!(v.get("meili_master_key").is_none());
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
    );
    assert_eq!(s.cache_dir, before);
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
fn test_mode_env_not_written_to_config_toml() {
    let dir = tempfile::tempdir().expect("tmp");
    let _guard = EnvGuard::with_config_dir(dir.path());
    let _tm = TestModeGuard::set(Some("1"));
    let mut s = AppSettings::default();
    s.workbench_knowledge_root = dir.path().join("wb");
    save(&s).expect("save");
    let text = fs::read_to_string(dir.path().join("config.toml")).expect("read config");
    assert!(!text.contains("test_mode"));
    assert!(!text.contains("TEST_MODE"));
    assert!(!text.contains("debug"));
}
