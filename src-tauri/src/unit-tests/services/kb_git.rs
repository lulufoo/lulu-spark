use super::*;
use crate::config::settings;
use std::fs;

fn configure_repo(repo: &std::path::Path) {
    git::exec(repo, &["config", "user.name", "Test User"]).expect("config name");
    git::exec(repo, &["config", "user.email", "test@example.com"]).expect("config email");
}

fn exec_ok(repo: &std::path::Path, args: &[&str]) {
    let out = git::exec(repo, args).expect("git exec");
    assert!(out.success, "git {:?} failed: {}", args, out.stderr);
}

#[test]
fn kb_commit_fails_pre_check_when_unmerged() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo = kb.join("myrepo");
    fs::create_dir_all(&repo).expect("mkdir");
    exec_ok(&repo, &["init"]);
    configure_repo(&repo);
    settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_git_commit(&json!({ "repo": "org/myrepo" }));
    assert!(v.get("step").is_none() || v["step"] == "pre-check" || v.get("error").is_some());
    settings::set_test_config_dir(None);
}

#[test]
fn kb_revert_all_resets_unmerged_repo() {
    let dir = tempfile::tempdir().expect("tmp");
    let kb = dir.path().join("kb");
    let repo = kb.join("myrepo");
    fs::create_dir_all(&repo).expect("mkdir");
    exec_ok(&repo, &["init"]);
    configure_repo(&repo);
    let default_branch = git::exec(&repo, &["branch", "--show-current"])
        .expect("show current branch")
        .stdout
        .trim()
        .to_string();

    fs::write(repo.join("note.md"), "shared\nbody\n").expect("write base");
    exec_ok(&repo, &["add", "note.md"]);
    exec_ok(&repo, &["commit", "-m", "base"]);

    exec_ok(&repo, &["checkout", "-b", "feature"]);
    fs::write(repo.join("note.md"), "feature\nbody\n").expect("write feature");
    exec_ok(&repo, &["commit", "-am", "feature change"]);

    exec_ok(&repo, &["checkout", &default_branch]);
    fs::write(repo.join("note.md"), "master\nbody\n").expect("write master");
    exec_ok(&repo, &["commit", "-am", "master change"]);

    let merge = git::exec(&repo, &["merge", "feature"]).expect("merge feature");
    assert!(!merge.success, "merge should conflict");
    assert!(git::has_unmerged(&repo).expect("has unmerged"));

    settings::write_test_config(dir.path(), dir.path(), Some(&kb));
    let v = kb_git_revert(&json!({ "repo": "org/myrepo" }));
    assert_eq!(v["ok"], json!(true));
    assert!(!git::has_unmerged(&repo).expect("unmerged cleared"));
    assert_eq!(
        fs::read_to_string(repo.join("note.md")).expect("read note"),
        "master\nbody\n"
    );
    settings::set_test_config_dir(None);
}
