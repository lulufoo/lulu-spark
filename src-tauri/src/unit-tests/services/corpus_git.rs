use super::*;
use crate::config::settings;
use std::fs;

fn with_git_corpus<F: FnOnce(&std::path::Path)>(f: F) {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir");
    git::exec(&corpus, &["init"]).expect("init");
    git::exec(&corpus, &["config", "user.email", "t@t.com"]).expect("e");
    git::exec(&corpus, &["config", "user.name", "t"]).expect("n");
    fs::write(corpus.join("f.md"), "a").expect("w");
    settings::write_test_config(dir.path(), &corpus, None);
    f(dir.path());
    settings::set_test_config_dir(None);
}

#[test]
fn corpus_commit_errors_when_corpus_not_git() {
    let dir = tempfile::tempdir().expect("tmp");
    let corpus = dir.path().join("corpus");
    fs::create_dir_all(&corpus).expect("mkdir");
    settings::write_test_config(dir.path(), &corpus, None);
    let v = corpus_git_commit(&json!({}));
    assert!(v.get("error").is_some());
    settings::set_test_config_dir(None);
}
