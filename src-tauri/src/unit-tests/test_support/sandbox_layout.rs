//! Tests for `test_support::sandbox_layout`: one sandbox = one dir
//! (`<HOME>/.config/lulu-spark-sandbox/<id>/{config.toml,cache/}`).

use std::fs;
use std::time::SystemTime;

use crate::config::settings::{
    self, default_cache_dir, load_prod_settings, prod_config_file_path,
    DEFAULT_SANDBOX_HTTP_PORT, DEFAULT_SANDBOX_MCP_PORT,
};
use crate::test_support::{with_sandbox_notes, TestSandbox};

fn prod_cache_dir_mtime() -> Option<SystemTime> {
    let path = default_cache_dir();
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn prod_config_mtime() -> Option<SystemTime> {
    fs::metadata(prod_config_file_path())
        .ok()
        .and_then(|m| m.modified().ok())
}

#[test]
fn test_sandbox_new_creates_isolated_three_roots() {
    let sandbox = TestSandbox::new();
    let cfg = settings::load().expect("load");
    let base = sandbox.sandbox_dir().join("cache");
    assert!(cfg.spark_root.starts_with(&base));
    assert!(cfg.knowledge_root.starts_with(&base));
    assert!(cfg.cache_dir.starts_with(&base));
    assert_ne!(cfg.cache_dir, default_cache_dir());
    assert_eq!(
        sandbox.config_file_path(),
        settings::config_file_path().expect("config path")
    );
    assert_eq!(
        sandbox.ports(),
        (DEFAULT_SANDBOX_HTTP_PORT, DEFAULT_SANDBOX_MCP_PORT)
    );
}


#[test]
fn write_test_config_persists_to_sandbox_config_toml() {
    let sandbox = TestSandbox::new();
    let path = settings::config_file_path().expect("cfg path");
    let text = fs::read_to_string(&path).expect("read config");
    assert!(text.contains("spark_root"));
    assert!(text.contains("knowledge_root"));
    assert!(text.contains("cache_dir"));
    assert!(path.ends_with("config.toml"));
    let _ = sandbox;
}

#[test]
fn test_config_writer_uses_explicit_target_path() {
    let sandbox = TestSandbox::new();
    let expected = sandbox.config_file_path();
    let (http_port, mcp_port) = sandbox.ports();
    settings::write_test_config_with_cache(
        &expected,
        sandbox.config_dir(),
        Some(sandbox.config_dir()),
        Some(&sandbox.config_dir().join("cache")),
        http_port,
        mcp_port,
    )
    .expect("write explicit sandbox config");
    assert!(expected.is_file());

    let text = fs::read_to_string(expected).expect("read explicit config");
    assert!(text.contains("cache_dir"));
    assert!(text.contains(&format!("http_port = {http_port}")));
    assert!(text.contains(&format!("mcp_port = {mcp_port}")));
}

#[test]
fn nested_test_sandbox_serializes_distinct_roots() {
    crate::test_support::with_config_test_serial(|| {
        let _outer = TestSandbox::new();
        let outer_wb = settings::load().expect("load").spark_root;
        {
            let inner = TestSandbox::new();
            let inner_wb = settings::load().expect("load").spark_root;
            assert_ne!(outer_wb, inner_wb);
            assert!(settings::config_file_path()
                .expect("cfg")
                .ends_with("config.toml"));
            drop(inner);
        }
        assert_eq!(
            settings::load().expect("load").spark_root,
            outer_wb
        );
    });
}

#[test]
fn lib_tests_do_not_touch_prod_cache_dir_mtime() {
    let before = prod_cache_dir_mtime();
    {
        let _sandbox = TestSandbox::new();
        with_sandbox_notes(true, |_dir, notes| {
            let _ = fs::create_dir_all(notes.join("annotations/ai"));
        });
    }
    let after = prod_cache_dir_mtime();
    assert_eq!(before, after);
}

#[test]
fn lib_tests_do_not_touch_prod_config_mtime() {
    let before = prod_config_mtime();
    {
        let _sandbox = TestSandbox::new();
    }
    let after = prod_config_mtime();
    assert_eq!(before, after);
}

#[test]
fn test_sandbox_records_prod_three_roots_at_new() {
    let sandbox = TestSandbox::new();
    // Prod roots are the fake prod under the sandbox HOME, not the machine prod.
    assert_eq!(
        sandbox.prod_spark_root(),
        sandbox.config_dir().join("prod-wb")
    );
    assert_eq!(
        sandbox.prod_knowledge_root(),
        sandbox.config_dir().join("prod-knowledge-clones")
    );
    assert_eq!(sandbox.prod_cache_dir(), sandbox.config_dir().join("prod-cache"));
    let _ = load_prod_settings();
}

#[test]
fn test_sandbox_write_paths_not_equal_prod_roots() {
    let sandbox = TestSandbox::new();
    let cfg = settings::load().expect("load");
    assert_ne!(
        cfg.spark_root,
        sandbox.prod_spark_root()
    );
    assert_ne!(cfg.cache_dir, sandbox.prod_cache_dir());
    assert_ne!(
        cfg.knowledge_root,
        sandbox.prod_knowledge_root()
    );
}

#[test]
fn assert_not_prod_path_rejects_prod_spark_root() {
    let sandbox = TestSandbox::new();
    let prod = sandbox.prod_spark_root();
    assert!(sandbox.assert_not_prod_path(&prod).is_err());
    assert!(sandbox
        .assert_not_prod_path(&prod.join("knowledge"))
        .is_err());
}

#[test]
fn assert_not_prod_path_rejects_prod_cache_dir() {
    let sandbox = TestSandbox::new();
    let prod = sandbox.prod_cache_dir();
    assert!(sandbox.assert_not_prod_path(&prod).is_err());
    assert!(sandbox
        .assert_not_prod_path(&prod.join("read_later.json"))
        .is_err());
}

#[test]
fn config_toml_lives_in_sandbox_id_dir() {
    let sandbox = TestSandbox::new();
    let id = match settings::runtime_mode().expect("mode") {
        settings::RuntimeMode::Sandbox { id } => id,
        settings::RuntimeMode::Prod => panic!("TestSandbox must run in sandbox mode"),
    };
    let expected_dir = sandbox
        .config_dir()
        .join(".config")
        .join("lulu-spark-sandbox")
        .join(&id);
    assert_eq!(sandbox.sandbox_dir(), expected_dir);
    assert_eq!(sandbox.config_file_path(), expected_dir.join("config.toml"));
    assert!(sandbox.config_file_path().is_file());
}

#[test]
fn sandbox_roots_all_start_with_sandbox_cache() {
    let sandbox = TestSandbox::new();
    let cache = sandbox.sandbox_dir().join("cache");
    assert_eq!(sandbox.cache_dir(), cache);
    for path in [
        sandbox.spark_root(),
        sandbox.knowledge_root(),
        sandbox.cache_dir(),
        sandbox.data_dir(),
    ] {
        assert!(path.starts_with(&cache), "{} outside {}", path.display(), cache.display());
    }
    assert_eq!(sandbox.data_dir(), cache.join("data"));
    assert_eq!(sandbox.knowledge_root(), cache.join("data").join("knowledge"));
}

#[test]
fn fake_prod_roots_stay_outside_sandbox_dir() {
    let sandbox = TestSandbox::new();
    let dir = sandbox.sandbox_dir();
    for prod in [
        sandbox.prod_spark_root(),
        sandbox.prod_knowledge_root(),
        sandbox.prod_cache_dir(),
    ] {
        assert!(!prod.starts_with(&dir));
        assert!(!dir.starts_with(prod));
    }
}

#[test]
fn two_sandboxes_use_distinct_dirs_removed_on_drop() {
    crate::test_support::with_config_test_serial(|| {
        let first = TestSandbox::new();
        let first_dir = first.sandbox_dir();
        let second = TestSandbox::new();
        let second_dir = second.sandbox_dir();
        assert_ne!(first_dir, second_dir);
        drop(second);
        assert!(!second_dir.exists());
        drop(first);
        assert!(!first_dir.exists());
    });
}
