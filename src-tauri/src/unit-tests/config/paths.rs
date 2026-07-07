use super::*;
use crate::config::settings;
use crate::test_support::TestSandbox;

#[test]
fn cache_dir_uses_settings_not_repo_dot_cache() {
    let _sandbox = TestSandbox::new();
    let got = cache_dir().expect("cache_dir");
    let cfg = settings::load().expect("load");
    assert_eq!(got, cfg.cache_dir);
    let root = repo_root().expect("repo");
    assert_ne!(got, root.join(".cache"));
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
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let dir = sediment_kb_dir().expect("dir");
    let cats = sediment_kb_categories_path().expect("cats");
    let repos = sediment_kb_repos_path().expect("repos");
    assert_eq!(dir, wb.join("sediment-kb"));
    assert_eq!(cats, dir.join("categories.json"));
    assert_eq!(repos, dir.join("repos.json"));
}

#[test]
fn read_later_path_under_workbench_knowledge_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let path = read_later_path().expect("read_later");
    assert_eq!(path, wb.join("read_later").join("read_later.json"));
}

#[test]
fn ssot_paths_use_configured_workbench_root_not_cache() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let sediment = sediment_kb_dir().expect("sediment");
    let read_later = read_later_path().expect("read_later");
    assert!(sediment.starts_with(&wb));
    assert!(read_later.starts_with(&wb));
    let cache = cache_dir().expect("cache");
    assert!(!sediment.starts_with(&cache));
    assert!(!read_later.starts_with(&cache));
}
