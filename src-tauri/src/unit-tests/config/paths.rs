use super::*;
use crate::config::settings;
use crate::test_support::{TestConfigEnv, TestSandbox};

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
fn spark_data_db_path_under_runtime_data_dir() {
    let sandbox = TestSandbox::new();
    let data = runtime_data_dir().expect("data");
    let got = spark_data_db_path().expect("db");
    assert_eq!(got, data.join("spark-data.sqlite"));
    sandbox.assert_not_prod_path(&got).expect("sandbox db");
}

#[test]
fn notes_draft_path_resolves_under_cache_drafts_notes() {
    let _sandbox = TestSandbox::new();
    let cache = cache_dir().expect("cache");
    let temp_id = "tmp-path-1";
    let got = notes_draft_path(temp_id).expect("notes_draft_path");
    assert_eq!(got, cache.join("drafts").join("notes").join(temp_id));
}

#[test]
fn doc_highlights_file_uses_md5_under_cache() {
    let _sandbox = TestSandbox::new();
    let cache = cache_dir().expect("cache");
    let got = doc_highlights_file("7e175ab16c747d8769bcd2cfdd6abc32").expect("file");
    assert_eq!(
        got,
        cache
            .join("doc-highlights")
            .join("7e175ab16c747d8769bcd2cfdd6abc32.json")
    );
    assert!(doc_highlights_file("not-hex").is_err());
    assert!(doc_highlights_file("../aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").is_err());
}

#[test]
fn notes_draft_path_rejects_empty_or_traversal_temp_id() {
    let _sandbox = TestSandbox::new();
    assert!(notes_draft_path("").is_err());
    assert!(notes_draft_path("..").is_err());
    assert!(notes_draft_path("a/../b").is_err());
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
fn read_later_path_under_runtime_data_dir() {
    let sandbox = TestSandbox::new();
    let path = read_later_path().expect("read_later");
    assert_eq!(
        path,
        sandbox.data_dir().join("read_later").join("read_later.json")
    );
    assert_ne!(
        path,
        sandbox.spark_root().join("read_later").join("read_later.json")
    );
}

#[test]
fn message_center_path_follows_sandbox_cache_dir() {
    let sandbox = TestSandbox::new();
    let path = message_center_path().expect("message_center");
    assert_eq!(
        path,
        settings::runtime_cache_dir()
            .expect("sandbox cache")
            .join("message_center")
            .join("message_center.json")
    );
    assert!(path.starts_with(sandbox.sandbox_dir().join("cache")));
    sandbox.assert_not_prod_path(&path).expect("sandbox path");
}

#[test]
fn mcp_channel_tools_path_under_spark_root() {
    let sandbox = TestSandbox::new();
    let path = mcp_channel_tools_path().expect("mcp_channel_tools");
    assert_eq!(path, sandbox.spark_root().join("mcp_channel_tools.json"));
}

#[test]
fn todo_tasks_path_under_spark_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.spark_root();
    let path = todo_tasks_path().expect("todo_tasks");
    assert_eq!(path, wb.join("todo_tasks").join("todo_tasks.json"));
    assert!(
        !path.to_string_lossy().contains("plan_tasks"),
        "storage path must not retain plan_tasks segment: {path:?}"
    );
}

#[test]
fn ssot_paths_use_configured_spark_root_not_cache() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.spark_root();
    let workspaces = workspaces_root().expect("workspaces");
    let read_later = read_later_path().expect("read_later");
    let todo_tasks = todo_tasks_path().expect("todo_tasks");
    let message_center = message_center_path().expect("message_center");
    let data = sandbox.data_dir();
    assert!(workspaces.starts_with(&data));
    assert!(read_later.starts_with(&data));
    assert!(todo_tasks.starts_with(&wb));
    // One sandbox = one dir: runtime data and message center live under `<id>/cache`.
    let cache = sandbox.sandbox_dir().join("cache");
    assert!(!workspaces.starts_with(&wb));
    assert!(!read_later.starts_with(&wb));
    assert!(workspaces.starts_with(&cache));
    assert!(read_later.starts_with(&cache));
    assert!(!message_center.starts_with(&wb));
    assert!(message_center.starts_with(&cache));
}

#[test]
fn todo_tasks_v2_dir_and_index_under_spark_root() {
    let _sandbox = TestSandbox::new();
    let wb = _sandbox.spark_root();
    let dir = todo_tasks_dir().expect("todo_tasks_dir");
    let index = todo_tasks_index_path().expect("todo_tasks_index_path");
    assert_eq!(dir, wb.join("todo_tasks"));
    assert_eq!(index, wb.join("todo_tasks").join("index.json"));
    assert!(
        !dir.to_string_lossy().contains("plan_tasks"),
        "dir must not retain plan_tasks segment: {dir:?}"
    );
}

#[test]
fn todo_tasks_v2_task_paths_resolve_under_tasks_directory() {
    let _sandbox = TestSandbox::new();
    let wb = _sandbox.spark_root();
    let master_id = "task_a1b2c3d4e5f6";
    let task_dir = todo_tasks_task_dir(master_id).expect("todo_tasks_task_dir");
    let sub_tasks = todo_tasks_sub_tasks_path(master_id).expect("todo_tasks_sub_tasks_path");
    let plan_md = todo_tasks_plan_md_path(master_id).expect("todo_tasks_plan_md_path");
    let expected_task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
    assert_eq!(task_dir, expected_task_dir);
    assert_eq!(sub_tasks, expected_task_dir.join("sub_tasks.json"));
    assert_eq!(plan_md, expected_task_dir.join("todo.md"));
    assert!(
        !plan_md.to_string_lossy().ends_with("plan.md"),
        "body file must be todo.md, not plan.md: {plan_md:?}"
    );
}

#[test]
fn cache_todo_tasks_v1_path_is_legacy_cache_file() {
    let _sandbox = TestSandbox::new();
    let cache = cache_dir().expect("cache");
    let path = cache_todo_tasks_v1_path().expect("cache_todo_tasks_v1_path");
    assert_eq!(path, cache.join("todo_tasks.json"));
    assert_ne!(path, todo_tasks_path().expect("todo_tasks_path"));
    assert!(
        !path.to_string_lossy().contains("plan_tasks"),
        "cache path must not retain plan_tasks segment: {path:?}"
    );
}

#[test]
fn todo_tasks_v2_task_path_helpers_reject_empty_master_task_id() {
    let _sandbox = TestSandbox::new();
    assert!(todo_tasks_task_dir("").is_err());
    assert!(todo_tasks_sub_tasks_path("").is_err());
    assert!(todo_tasks_plan_md_path("").is_err());
}

#[test]
fn todo_tasks_v2_path_helpers_err_when_settings_unavailable() {
    let dir = tempfile::tempdir().expect("tmp");
    let env = TestConfigEnv::sandbox(dir.path(), "badcfg");
    let config_path = env.config_file_path();
    let cfg_dir = config_path.parent().expect("config parent").to_path_buf();
    std::fs::create_dir_all(&cfg_dir).expect("mkdir");
    std::fs::write(config_path, "not valid toml {{{").expect("write corrupt config");

    assert!(matches!(todo_tasks_dir(), Err(PathsError::Settings(_))));
    assert!(matches!(
        todo_tasks_index_path(),
        Err(PathsError::Settings(_))
    ));
    assert!(matches!(
        cache_todo_tasks_v1_path(),
        Err(PathsError::Settings(_))
    ));
    assert!(matches!(
        todo_tasks_task_dir("task_abc"),
        Err(PathsError::Settings(_))
    ));
}

#[test]
fn todo_tasks_categories_path_under_todo_tasks_dir_not_knowledge() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.spark_root();
    let dir = todo_tasks_dir().expect("todo_tasks_dir");
    let cats = todo_tasks_categories_path().expect("todo_tasks_categories_path");
    assert_eq!(cats, dir.join("categories.json"));
    assert_eq!(cats, wb.join("todo_tasks").join("categories.json"));
    assert_ne!(
        cats,
        workspaces_root().expect("workspaces").join("categories.json"),
        "todo categories registry must not reuse the workspaces path"
    );
}
