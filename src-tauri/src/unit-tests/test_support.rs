//! Tests for `crate::test_support` helpers (t2, t3).

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::config::settings::{self, default_cache_dir};
use crate::test_support::{TestSandbox, with_corpus, with_test_config_dir};

fn prod_cache_dir_mtime() -> Option<SystemTime> {
    let path = default_cache_dir();
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
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
fn test_sandbox_drop_clears_config_dir_override() {
    let outer = tempfile::tempdir().expect("tmp");
    settings::set_test_config_dir(Some(outer.path().to_path_buf()));
    {
        let _sandbox = TestSandbox::new();
        assert_ne!(
            settings::config_file_path().parent(),
            Some(outer.path())
        );
    }
    assert_eq!(
        settings::config_file_path().parent(),
        Some(outer.path())
    );
    settings::set_test_config_dir(None);
}

#[test]
fn write_test_config_persists_three_roots() {
    let dir = tempfile::tempdir().expect("tmp");
    let wb = dir.path().join("wb");
    let corpus = dir.path().join("corpus");
    let cache = dir.path().join("cache");
    fs::create_dir_all(&wb).expect("mkdir wb");
    fs::create_dir_all(&corpus).expect("mkdir corpus");
    fs::create_dir_all(&cache).expect("mkdir cache");

    settings::write_test_config_with_cache(dir.path(), &wb, Some(&corpus), Some(&cache));
    settings::set_test_config_dir(Some(dir.path().to_path_buf()));
    let text = fs::read_to_string(dir.path().join("config.toml")).expect("read config");
    assert!(text.contains("workbench_knowledge_root"));
    assert!(text.contains("knowledge_corpus_root"));
    assert!(text.contains("cache_dir"));
    let loaded = settings::load().expect("load");
    assert_eq!(loaded.workbench_knowledge_root, wb);
    assert_eq!(loaded.knowledge_corpus_root, corpus);
    assert_eq!(loaded.cache_dir, cache);
    settings::set_test_config_dir(None);
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
fn nested_test_sandbox_inner_overrides_outer_restores() {
    let outer = TestSandbox::new();
    let outer_cfg = settings::config_file_path();
    {
        let inner = TestSandbox::new();
        assert_ne!(settings::config_file_path(), outer_cfg);
        assert!(settings::config_file_path().starts_with(inner.config_dir()));
        drop(inner);
    }
    assert_eq!(settings::config_file_path(), outer_cfg);
}

#[test]
fn lib_tests_do_not_touch_prod_cache_dir_mtime() {
    let before = prod_cache_dir_mtime();
    {
        let _sandbox = TestSandbox::new();
        with_corpus(true, |_dir, corpus| {
            let _ = fs::create_dir_all(corpus.join("annotations/ai"));
        });
    }
    let after = prod_cache_dir_mtime();
    assert_eq!(before, after);
}

#[test]
fn with_test_config_dir_sets_and_clears_config_dir() {
    let mut inner_path = None::<PathBuf>;
    with_test_config_dir(|p| {
        inner_path = Some(p.to_path_buf());
        assert_eq!(settings::config_file_path().parent(), Some(p));
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
