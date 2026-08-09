//! Tests for `crate::test_support` helpers.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::config::settings::{self, default_cache_dir, load_prod_settings, prod_config_file_path};
use crate::test_support::{TestSandbox, with_corpus, with_sandbox_corpus, with_test_config_dir};

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
    let base = sandbox.config_dir();
    assert!(cfg.workbench_knowledge_root.starts_with(base));
    assert!(cfg.knowledge_corpus_root.starts_with(base));
    assert!(cfg.cache_dir.starts_with(base));
    assert_ne!(cfg.cache_dir, default_cache_dir());
}

#[test]
fn write_test_config_persists_to_sandbox_config_toml() {
    let sandbox = TestSandbox::new();
    let path = settings::config_file_path().expect("cfg path");
    let text = fs::read_to_string(&path).expect("read config");
    assert!(text.contains("workbench_knowledge_root"));
    assert!(text.contains("knowledge_corpus_root"));
    assert!(text.contains("cache_dir"));
    assert!(path.ends_with("config.toml"));
    let _ = sandbox;
}

#[test]
fn with_corpus_sets_sandbox_cache_dir_not_prod() {
    with_corpus(false, |_dir, _corpus| {
        let cfg = settings::load().expect("load");
        assert!(cfg.cache_dir.starts_with(_dir.path()));
        assert_ne!(cfg.cache_dir, default_cache_dir());
        assert_ne!(cfg.workbench_knowledge_root, default_cache_dir());
    });
}

#[test]
fn nested_test_sandbox_serializes_distinct_roots() {
    crate::test_support::with_config_test_serial(|| {
        let outer = TestSandbox::new();
        let outer_wb = settings::load().expect("load").workbench_knowledge_root;
        {
            let inner = TestSandbox::new();
            let inner_wb = settings::load().expect("load").workbench_knowledge_root;
            assert_ne!(outer_wb, inner_wb);
            assert!(settings::config_file_path()
                .expect("cfg")
                .ends_with("config.toml"));
            drop(inner);
        }
        assert_eq!(
            settings::load().expect("load").workbench_knowledge_root,
            outer_wb
        );
    });
}

#[test]
fn lib_tests_do_not_touch_prod_cache_dir_mtime() {
    let before = prod_cache_dir_mtime();
    {
        let _sandbox = TestSandbox::new();
        with_sandbox_corpus(true, |_dir, corpus| {
            let _ = fs::create_dir_all(corpus.join("annotations/ai"));
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
fn with_test_config_dir_uses_isolated_config_toml() {
    let mut inner_path = None::<PathBuf>;
    with_test_config_dir(|p| {
        inner_path = Some(p.to_path_buf());
        let cfg = settings::config_file_path().expect("cfg");
        assert!(cfg.starts_with(p));
        assert!(cfg.ends_with("config.toml"));
    });
    assert!(inner_path.is_some());
}

#[test]
fn with_corpus_false_creates_annotations_only() {
    with_corpus(false, |_dir, corpus| {
        assert!(corpus.join("annotations").is_dir());
        assert!(!corpus.join("annotations/ai").exists());
    });
}

#[test]
fn with_corpus_true_creates_ai_subdir() {
    with_corpus(true, |_dir, corpus| {
        assert!(corpus.join("annotations/ai").is_dir());
    });
}

#[test]
fn test_sandbox_records_prod_three_roots_at_new() {
    let sandbox = TestSandbox::new();
    // Prod roots are the fake prod under the sandbox HOME, not the machine prod.
    assert_eq!(
        sandbox.prod_workbench_knowledge_root(),
        sandbox.config_dir().join("prod-wb")
    );
    assert_eq!(
        sandbox.prod_knowledge_corpus_root(),
        sandbox.config_dir().join("prod-corpus")
    );
    assert_eq!(sandbox.prod_cache_dir(), sandbox.config_dir().join("prod-cache"));
    let _ = load_prod_settings();
}

#[test]
fn test_sandbox_write_paths_not_equal_prod_roots() {
    let sandbox = TestSandbox::new();
    let cfg = settings::load().expect("load");
    assert_ne!(
        cfg.workbench_knowledge_root,
        sandbox.prod_workbench_knowledge_root()
    );
    assert_ne!(cfg.cache_dir, sandbox.prod_cache_dir());
    assert_ne!(
        cfg.knowledge_corpus_root,
        sandbox.prod_knowledge_corpus_root()
    );
}

#[test]
fn assert_not_prod_path_rejects_prod_workbench_knowledge_root() {
    let sandbox = TestSandbox::new();
    let prod = sandbox.prod_workbench_knowledge_root();
    assert!(sandbox.assert_not_prod_path(&prod).is_err());
    assert!(sandbox
        .assert_not_prod_path(&prod.join("sediment-kb"))
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
