//! Tests for `crate::test_support` helpers (t2).

use std::path::PathBuf;

use crate::config::settings;
use crate::test_support::{with_corpus, with_test_config_dir};

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
