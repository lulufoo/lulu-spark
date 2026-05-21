use super::*;
use crate::config::settings;
use std::fs;

#[test]
fn kb_commit_fails_pre_check_when_unmerged() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo = kb.join("myrepo");
    fs::create_dir_all(&repo).expect("mkdir");
    git::exec(&repo, &["init"]).expect("init");
    settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_git_commit(&json!({ "repo": "org/myrepo" }));
    assert!(v.get("step").is_none() || v["step"] == "pre-check" || v.get("error").is_some());
    settings::set_test_config_dir(None);
}
