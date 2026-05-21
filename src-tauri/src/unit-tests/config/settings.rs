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
