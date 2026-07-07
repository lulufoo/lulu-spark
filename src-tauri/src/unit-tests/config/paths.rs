use super::*;
use std::fs;

use crate::test_support::with_test_config_dir;

#[test]
fn cache_dir_uses_settings_not_repo_dot_cache() {
    with_test_config_dir(|cfg| {
        let custom = cfg.join("custom-cache");
        fs::write(
            cfg.join("config.toml"),
            format!(r#"cache_dir = "{}""#, custom.display()),
        )
        .expect("write");
        let got = cache_dir().expect("cache_dir");
        assert_eq!(got, custom);
        let root = repo_root().expect("repo");
        assert_ne!(got, root.join(".cache"));
    });
}

#[test]
fn repo_root_matches_cargo_manifest_parent() {
    let root = repo_root().expect("repo_root");
    let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent")
        .to_path_buf();
    assert_eq!(root, expected);
}

#[test]
fn sediment_kb_paths_under_workbench_knowledge_root() {
    with_test_config_dir(|cfg| {
        let wb = cfg.join("workbench-kb");
        let cache = cfg.join("custom-cache");
        fs::write(
            cfg.join("config.toml"),
            format!(
                r#"workbench_knowledge_root = "{}"
cache_dir = "{}""#,
                wb.display(),
                cache.display()
            ),
        )
        .expect("write");
        let dir = sediment_kb_dir().expect("dir");
        let cats = sediment_kb_categories_path().expect("cats");
        let repos = sediment_kb_repos_path().expect("repos");
        assert_eq!(dir, wb.join("sediment-kb"));
        assert_eq!(cats, dir.join("categories.json"));
        assert_eq!(repos, dir.join("repos.json"));
        assert_ne!(dir, cache.join("sediment-kb"));
    });
}

#[test]
fn read_later_path_under_workbench_knowledge_root() {
    with_test_config_dir(|cfg| {
        let wb = cfg.join("workbench-kb");
        let cache = cfg.join("custom-cache");
        fs::write(
            cfg.join("config.toml"),
            format!(
                r#"workbench_knowledge_root = "{}"
cache_dir = "{}""#,
                wb.display(),
                cache.display()
            ),
        )
        .expect("write");
        let path = read_later_path().expect("read_later");
        assert_eq!(path, wb.join("read_later").join("read_later.json"));
        assert_ne!(path, cache.join("read_later.json"));
    });
}

#[test]
fn ssot_paths_use_configured_workbench_root_not_cache() {
    with_test_config_dir(|cfg| {
        let wb = cfg.join("corpus-root");
        fs::write(
            cfg.join("config.toml"),
            format!(r#"workbench_knowledge_root = "{}""#, wb.display()),
        )
        .expect("write");
        let sediment = sediment_kb_dir().expect("sediment");
        let read_later = read_later_path().expect("read_later");
        assert!(sediment.starts_with(&wb));
        assert!(read_later.starts_with(&wb));
        let cache = cache_dir().expect("cache");
        assert!(!sediment.starts_with(&cache));
        assert!(!read_later.starts_with(&cache));
    });
}
