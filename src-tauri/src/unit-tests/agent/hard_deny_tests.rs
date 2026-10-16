use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;

use crate::agent::fence::spark::expand_for_business_key;
use crate::agent::fence::{stored_path, validate_stage_file};
use crate::agent::hard_deny::HardDenyList;
use crate::agent::r#loop;
use crate::config::paths;
use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
use crate::test_support::TestSandbox;

fn same_path(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn with_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    registry::clear_for_tests();
    registry::seed_defaults();
    f(&sandbox);
}

fn plant_demo_repo(_sandbox: &TestSandbox) {
    crate::services::sediment_kb::add_directory("demo").expect("demo");
}

#[test]
fn spark_defaults_lists_data_dir_and_git() {
    with_sandbox(|sandbox| {
        plant_demo_repo(sandbox);
        let list = HardDenyList::spark_defaults().expect("list");
        let data = paths::runtime_data_dir().expect("data");
        let wb = stored_path(paths::spark_root().expect("wb"));
        let clone_git = stored_path(paths::knowledge_root().expect("kb").join("demo")).join(".git");
        assert!(list.roots().iter().any(|p| same_path(p, &data)));
        assert!(list.roots().contains(&wb.join(".git")));
        assert!(list.roots().contains(&clone_git));
    });
}

#[test]
fn contains_spark_data_and_git() {
    with_sandbox(|sandbox| {
        plant_demo_repo(sandbox);
        let list = HardDenyList::spark_defaults().expect("list");
        let note = sandbox.data_dir().join("notes").join("n1.md");
        fs::create_dir_all(note.parent().unwrap()).expect("notes");
        fs::write(&note, "body").expect("note");
        assert!(list.contains(&note), "Spark data must hard-deny");

        let git = sandbox.spark_root().join(".git").join("config");
        fs::create_dir_all(git.parent().unwrap()).expect("git");
        fs::write(&git, "x").expect("git file");
        assert!(list.contains(&git), ".git must hard-deny");
    });
}

#[test]
fn staged_grant_does_not_override_hard_deny() {
    with_sandbox(|sandbox| {
        let note = sandbox.data_dir().join("notes").join("n2.md");
        fs::create_dir_all(note.parent().unwrap()).expect("notes");
        fs::write(&note, "body").expect("note");
        let list = HardDenyList::spark_defaults().expect("list");
        assert!(list.contains(&note));

        let mut fence = expand_for_business_key(SEEDED_BUSINESS_KEY)
            .expect("fence")
            .with_session_scratch("spark_chat_abc")
            .expect("scratch");
        fence.grant_staged_file(stored_path(note.clone()));
        assert!(list.contains(&note));
        assert!(!fence.allows_read(&note));
        assert!(!fence.allows_write(&note));
        let err = validate_stage_file(note.to_str().unwrap(), &fence).expect_err("stage");
        assert!(err.contains("read fence"), "{err}");
    });
}

#[test]
fn symlink_into_data_is_hard_denied() {
    with_sandbox(|sandbox| {
        let note = sandbox.data_dir().join("notes").join("n3.md");
        fs::create_dir_all(note.parent().unwrap()).expect("notes");
        fs::write(&note, "body").expect("note");
        let link = sandbox.config_dir().join("external").join("alias.md");
        fs::create_dir_all(link.parent().unwrap()).expect("external");
        symlink(&note, &link).expect("symlink");

        let list = HardDenyList::spark_defaults().expect("list");
        assert!(
            list.contains(&link),
            "symlink into Spark data must hard-deny, link={link:?} target={note:?}"
        );
        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY).expect("fence");
        assert!(!fence.allows_read(&link));
        assert!(!fence.allows_write(&link));
    });
}

#[test]
fn fence_read_deny_is_the_hard_deny_list() {
    with_sandbox(|sandbox| {
        plant_demo_repo(sandbox);
        let list = HardDenyList::spark_defaults().expect("list");
        let fence = expand_for_business_key(SEEDED_BUSINESS_KEY).expect("fence");
        assert_eq!(fence.read_deny, list.roots());
    });
}
