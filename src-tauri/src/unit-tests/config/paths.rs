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
