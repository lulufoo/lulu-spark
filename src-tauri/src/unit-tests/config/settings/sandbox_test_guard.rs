//! Tests for `config::settings::sandbox_test_guard` (cfg(test) write guards).

use std::fs;

use crate::config::settings;

#[test]
fn config_write_guard_rejects_dot_dot_and_symlink_aliases() {
    let temp_machine_home = tempfile::tempdir().expect("temp machine home");
    let protected = temp_machine_home
        .path()
        .join(".config/lulu-spark/config.toml");
    fs::create_dir_all(protected.parent().expect("protected parent")).expect("mkdir");
    fs::write(&protected, "cache_dir = \"safe\"").expect("create machine config");

    let alias_parent = protected.parent().expect("protected parent").join("alias");
    fs::create_dir_all(&alias_parent).expect("create alias parent");
    let dot_dot_alias = alias_parent
        .join("..")
        .join(protected.file_name().expect("config file name"));
    assert!(settings::reject_write_to_protected_config(&dot_dot_alias, &protected).is_err());

    #[cfg(unix)]
    {
        let symlink_alias = temp_machine_home.path().join("config.toml");
        std::os::unix::fs::symlink(&protected, &symlink_alias).expect("create symlink");
        assert!(settings::reject_write_to_protected_config(&symlink_alias, &protected).is_err());
    }
}

#[test]
fn atomic_config_write_replaces_hard_link_without_mutating_protected_file() {
    let temp_machine_home = tempfile::tempdir().expect("temp machine home");
    let protected = temp_machine_home
        .path()
        .join(".config/lulu-spark/config.toml");
    fs::create_dir_all(protected.parent().expect("protected parent")).expect("mkdir");
    fs::write(&protected, "cache_dir = \"safe\"").expect("create machine config");
    let protected_before = fs::read_to_string(&protected).expect("read protected");
    let hard_link_alias = protected
        .parent()
        .expect("protected parent")
        .join("hard-link-config.toml");
    fs::hard_link(&protected, &hard_link_alias).expect("create hard link");

    settings::atomic_write_test_config(&hard_link_alias, "cache_dir = \"replacement\"")
        .expect("atomic replacement");

    assert_eq!(
        fs::read_to_string(&protected).expect("read protected"),
        protected_before
    );
    assert_ne!(
        fs::read_to_string(&hard_link_alias).expect("read replacement"),
        protected_before
    );
}
