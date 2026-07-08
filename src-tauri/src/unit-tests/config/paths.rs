use super::*;
use crate::config::settings;
use crate::test_support::TestSandbox;

#[test]
fn cache_dir_uses_settings_not_repo_dot_cache() {
    let _sandbox = TestSandbox::new();
    let got = cache_dir().expect("cache_dir");
    let cfg = settings::load().expect("load");
    assert_eq!(got, cfg.cache_dir);
    let root = repo_root().expect("repo");
    assert_ne!(got, root.join(".cache"));
}

#[test]
fn repo_root_matches_cargo_manifest_parent() {
    let root = repo_root().expect("repo_root");
    let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent")
        .to_path_buf();
    assert_eq!(root, expected);
}

#[test]
fn sediment_kb_paths_under_workbench_knowledge_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let dir = sediment_kb_dir().expect("dir");
    let cats = sediment_kb_categories_path().expect("cats");
    let repos = sediment_kb_repos_path().expect("repos");
    assert_eq!(dir, wb.join("sediment-kb"));
    assert_eq!(cats, dir.join("categories.json"));
    assert_eq!(repos, dir.join("repos.json"));
}

#[test]
fn read_later_path_under_workbench_knowledge_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let path = read_later_path().expect("read_later");
    assert_eq!(path, wb.join("read_later").join("read_later.json"));
}

#[test]
fn plan_tasks_path_under_workbench_knowledge_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let path = plan_tasks_path().expect("plan_tasks");
    assert_eq!(path, wb.join("plan_tasks").join("plan_tasks.json"));
}

#[test]
fn ssot_paths_use_configured_workbench_root_not_cache() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_knowledge_root();
    let sediment = sediment_kb_dir().expect("sediment");
    let read_later = read_later_path().expect("read_later");
    let plan_tasks = plan_tasks_path().expect("plan_tasks");
    assert!(sediment.starts_with(&wb));
    assert!(read_later.starts_with(&wb));
    assert!(plan_tasks.starts_with(&wb));
    let cache = cache_dir().expect("cache");
    assert!(!sediment.starts_with(&cache));
    assert!(!read_later.starts_with(&cache));
    assert!(!plan_tasks.starts_with(&cache));
}

#[test]
fn plan_tasks_v2_dir_and_index_under_workbench_knowledge_root() {
    let _sandbox = TestSandbox::new();
    let wb = _sandbox.workbench_knowledge_root();
    let dir = plan_tasks_dir().expect("plan_tasks_dir");
    let index = plan_tasks_index_path().expect("plan_tasks_index_path");
    assert_eq!(dir, wb.join("plan_tasks"));
    assert_eq!(index, wb.join("plan_tasks").join("index.json"));
}

#[test]
fn plan_tasks_v2_task_paths_resolve_under_tasks_directory() {
    let _sandbox = TestSandbox::new();
    let wb = _sandbox.workbench_knowledge_root();
    let master_id = "task_a1b2c3d4e5f6";
    let task_dir = plan_tasks_task_dir(master_id).expect("plan_tasks_task_dir");
    let sub_tasks = plan_tasks_sub_tasks_path(master_id).expect("plan_tasks_sub_tasks_path");
    let plan_md = plan_tasks_plan_md_path(master_id).expect("plan_tasks_plan_md_path");
    let expected_task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
    assert_eq!(task_dir, expected_task_dir);
    assert_eq!(sub_tasks, expected_task_dir.join("sub_tasks.json"));
    assert_eq!(plan_md, expected_task_dir.join("plan.md"));
}

#[test]
fn cache_plan_tasks_v1_path_is_legacy_cache_file() {
    let _sandbox = TestSandbox::new();
    let cache = cache_dir().expect("cache");
    let path = cache_plan_tasks_v1_path().expect("cache_plan_tasks_v1_path");
    assert_eq!(path, cache.join("plan_tasks.json"));
    assert_ne!(path, plan_tasks_path().expect("plan_tasks_path"));
}

#[test]
fn plan_tasks_v2_task_path_helpers_reject_empty_master_task_id() {
    let _sandbox = TestSandbox::new();
    assert!(plan_tasks_task_dir("").is_err());
    assert!(plan_tasks_sub_tasks_path("").is_err());
    assert!(plan_tasks_plan_md_path("").is_err());
}

#[test]
fn plan_tasks_v2_path_helpers_err_when_settings_unavailable() {
    crate::test_support::with_config_test_serial(|| {
        let dir = tempfile::tempdir().expect("tmp");
        let prev = std::env::var("LULU_WB_CONFIG_DIR").ok();
        unsafe {
            std::env::set_var("LULU_WB_CONFIG_DIR", dir.path());
        }
        std::fs::write(
            dir.path().join(settings::PROD_CONFIG_FILE_NAME),
            "not valid toml {{{",
        )
        .expect("write corrupt config");

        assert!(matches!(
            plan_tasks_dir(),
            Err(PathsError::Settings(_))
        ));
        assert!(matches!(
            plan_tasks_index_path(),
            Err(PathsError::Settings(_))
        ));
        assert!(matches!(
            cache_plan_tasks_v1_path(),
            Err(PathsError::Settings(_))
        ));
        assert!(matches!(
            plan_tasks_task_dir("task_abc"),
            Err(PathsError::Settings(_))
        ));

        match prev {
            Some(v) => unsafe { std::env::set_var("LULU_WB_CONFIG_DIR", v) },
            None => unsafe { std::env::remove_var("LULU_WB_CONFIG_DIR") },
        }
    });
}
