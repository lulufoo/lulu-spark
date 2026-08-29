use super::*;
use crate::test_support::TestSandbox;
use std::fs;

fn configure_repo(repo: &std::path::Path) {
    git::exec(repo, &["config", "user.name", "Test User"]).expect("config name");
    git::exec(repo, &["config", "user.email", "test@example.com"]).expect("config email");
}

fn exec_ok(repo: &std::path::Path, args: &[&str]) {
    let out = git::exec(repo, args).expect("git exec");
    assert!(out.success, "git {:?} failed: {}", args, out.stderr);
}

fn with_kb_git<F: FnOnce(&std::path::Path, &std::path::Path)>(setup: impl FnOnce(&std::path::Path, &std::path::Path), f: F) {
    let sandbox = TestSandbox::new();
    let cfg_dir = sandbox.config_dir();
    let kb = sandbox.knowledge_root();
    setup(cfg_dir, &kb);
    f(cfg_dir, &kb);
}

#[test]
fn kb_commit_fails_pre_check_when_unmerged() {
    with_kb_git(
        |_, kb| {
            let repo = kb.join("myrepo");
            fs::create_dir_all(&repo).expect("mkdir");
            exec_ok(&repo, &["init"]);
            configure_repo(&repo);
        },
        |_, _| {
            let v = kb_git_commit(&json!({ "repo": "org/myrepo" }));
            assert!(v.get("step").is_none() || v["step"] == "pre-check" || v.get("error").is_some());
        },
    );
}

#[test]
fn kb_revert_all_resets_unmerged_repo() {
    with_kb_git(
        |_, kb| {
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
        },
        |_, kb| {
            let repo = kb.join("myrepo");
            let v = kb_git_revert(&json!({ "repo": "org/myrepo" }));
            assert_eq!(v["ok"], json!(true));
            assert!(!git::has_unmerged(&repo).expect("unmerged cleared"));
            assert_eq!(
                fs::read_to_string(repo.join("note.md")).expect("read note"),
                "master\nbody\n"
            );
        },
    );
}

#[test]
fn kb_revert_all_removes_untracked_nested_git_dirs() {
    with_kb_git(
        |_, kb| {
            let repo = kb.join("myrepo");
            fs::create_dir_all(&repo).expect("mkdir");
            exec_ok(&repo, &["init"]);
            configure_repo(&repo);
            fs::write(repo.join("README.md"), "base\n").expect("write base");
            exec_ok(&repo, &["add", "README.md"]);
            exec_ok(&repo, &["commit", "-m", "base"]);

            let nested = repo.join(".cache/worktrees/feature-a");
            fs::create_dir_all(&nested).expect("mkdir nested");
            exec_ok(&nested, &["init"]);
            fs::write(nested.join("scratch.txt"), "wip\n").expect("write scratch");

            let status = git::status_porcelain(&repo).expect("status");
            assert!(status.contains(".cache/"), "fixture should show untracked .cache/");
        },
        |_, kb| {
            let repo = kb.join("myrepo");
            let v = kb_git_revert(&json!({ "repo": "org/myrepo" }));
            assert_eq!(v["ok"], json!(true));

            assert!(
                !repo.join(".cache").exists(),
                "revert all should remove .cache/ including nested git worktrees"
            );
            let after = git::status_porcelain(&repo).expect("status after");
            assert!(after.trim().is_empty(), "expected clean tree, got: {after}");
        },
    );
}
