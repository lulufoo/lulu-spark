//! Tests for `config::settings::runtime_mode`.

use super::{
    clear_runtime_plane, install_runtime_plane, resolve_cache_dir, resolve_real_user_home,
    resolve_runtime_mode, resolve_sandbox_dir, runtime_cache_dir, runtime_config_dir,
    runtime_normalize_prod_paths, RuntimeMode,
};
use crate::config::settings::default_cache_dir;
use crate::test_support::{TestConfigEnv, TestSandbox};

fn current_sandbox_id() -> String {
    match resolve_runtime_mode().expect("mode") {
        RuntimeMode::Sandbox { id } => id,
        RuntimeMode::Prod => panic!("expected sandbox mode"),
    }
}

#[test]
fn resolve_runtime_mode_returns_prod_when_unset() {
    let home = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::prod(home.path());
    assert_eq!(resolve_runtime_mode().expect("mode"), RuntimeMode::Prod);
}

#[test]
fn resolve_runtime_mode_returns_sandbox_with_valid_id() {
    let home = tempfile::tempdir().expect("tmp");
    for flag in ["true", "1"] {
        let _env = TestConfigEnv::raw(home.path(), Some(flag), Some("abc_1-Z"));
        assert_eq!(
            resolve_runtime_mode().expect("mode"),
            RuntimeMode::Sandbox {
                id: "abc_1-Z".into()
            }
        );
    }
}

#[test]
fn resolve_runtime_mode_rejects_missing_id() {
    let home = tempfile::tempdir().expect("tmp");
    for id in [None, Some(""), Some("   ")] {
        let _env = TestConfigEnv::raw(home.path(), Some("true"), id);
        assert!(resolve_runtime_mode().is_err(), "id {id:?} must be rejected");
    }
}

#[test]
fn resolve_runtime_mode_rejects_invalid_id() {
    let home = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::sandbox(home.path(), "bad id/..");
    assert!(resolve_runtime_mode().is_err());
}

#[test]
fn resolve_runtime_mode_rejects_unknown_value() {
    let home = tempfile::tempdir().expect("tmp");
    for flag in ["yes", "TRUE", "0", "false", ""] {
        let _env = TestConfigEnv::raw(home.path(), Some(flag), Some("abc"));
        assert!(
            resolve_runtime_mode().is_err(),
            "flag {flag:?} must be rejected"
        );
    }
}

#[test]
fn resolve_cache_dir_returns_sandbox_cache() {
    let sandbox = TestSandbox::new();
    let id = current_sandbox_id();
    let expected = sandbox
        .config_dir()
        .join(".config/lulu-spark-sandbox")
        .join(&id)
        .join("cache");
    assert_eq!(resolve_cache_dir().expect("cache"), expected);
    assert_eq!(runtime_cache_dir().expect("plane"), expected);
    assert_eq!(runtime_config_dir().expect("config"), sandbox.sandbox_dir());
    assert!(!runtime_normalize_prod_paths().expect("normalize"));
    assert_eq!(resolve_sandbox_dir(&id), sandbox.sandbox_dir());
}

#[test]
fn resolve_cache_dir_returns_default_in_prod_under_temp_home() {
    let home = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::prod(home.path());
    assert_eq!(resolve_cache_dir().expect("cache"), default_cache_dir());
    assert_eq!(runtime_cache_dir().expect("plane"), default_cache_dir());
    assert_eq!(
        runtime_config_dir().expect("config"),
        home.path().join(".config").join("lulu-spark")
    );
    assert!(runtime_normalize_prod_paths().expect("normalize"));
}

#[test]
#[should_panic(expected = "real HOME")]
fn install_runtime_plane_panics_in_prod_on_real_home_during_tests() {
    let real_home = resolve_real_user_home().expect("passwd home");
    let _env = TestConfigEnv::prod(&real_home);
}

#[test]
fn invalid_mode_clears_runtime_plane() {
    let home = tempfile::tempdir().expect("tmp");
    let _env = TestConfigEnv::raw(home.path(), Some("yes"), Some("abc"));
    assert!(runtime_cache_dir().is_err());
}

#[test]
fn nested_env_restores_parent_plane() {
    let home = tempfile::tempdir().expect("tmp");
    let outer = TestConfigEnv::prod(home.path());
    let outer_cache = runtime_cache_dir().expect("outer");
    {
        let _inner = TestConfigEnv::sandbox(home.path(), "nested-plane");
        let inner = runtime_cache_dir().expect("inner");
        assert_ne!(inner, outer_cache);
        assert!(inner.ends_with("lulu-spark-sandbox/nested-plane/cache"));
    }
    assert_eq!(runtime_cache_dir().expect("restored"), outer_cache);
    drop(outer);
    assert!(runtime_cache_dir().is_err());
}

#[test]
fn clear_runtime_plane_drops_cache() {
    let _sandbox = TestSandbox::new();
    assert!(runtime_cache_dir().is_ok());
    clear_runtime_plane();
    assert!(runtime_cache_dir().is_err());
    install_runtime_plane().expect("reinstall");
    assert!(runtime_cache_dir().is_ok());
}

