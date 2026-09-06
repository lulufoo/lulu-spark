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
fn knowledge_doc_map_path_under_cache_dir() {
    let sandbox = TestSandbox::new();
    let cache = cache_dir().expect("cache");
    let got = knowledge_doc_map_path().expect("map");
    assert_eq!(got, cache.join("knowledge-doc-map.json"));
    sandbox.assert_not_prod_path(&got).expect("sandbox map");
}

#[test]
fn knowledge_hide_patterns_path_under_cache_dir() {
    let sandbox = TestSandbox::new();
    let cache = cache_dir().expect("cache");
    let got = knowledge_hide_patterns_path().expect("hide");
    assert_eq!(got, cache.join("knowledge-hide-patterns.json"));
    sandbox.assert_not_prod_path(&got).expect("sandbox hide");
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
fn notes_categories_path_under_notes_not_knowledge() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let path = notes_categories_path().expect("notes cats");
    assert_eq!(path, wb.join("notes").join("categories.json"));
    assert_ne!(path, sediment_kb_categories_path().expect("kb cats"));
}

#[test]
fn sediment_kb_paths_under_workbench_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let dir = sediment_kb_dir().expect("dir");
    let cats = sediment_kb_categories_path().expect("cats");
    let repos = sediment_kb_repos_path().expect("repos");
    assert_eq!(dir, wb.join("knowledge"));
    assert_eq!(cats, dir.join("categories.json"));
    assert_eq!(repos, dir.join("repos.json"));
}

#[test]
fn read_later_path_under_workbench_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let path = read_later_path().expect("read_later");
    assert_eq!(path, wb.join("read_later").join("read_later.json"));
}

#[test]
fn message_center_path_under_workbench_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let path = message_center_path().expect("message_center");
    assert_eq!(
        path,
        wb.join("message_center").join("message_center.json")
    );
    sandbox.assert_not_prod_path(&path).expect("sandbox path");
}

#[test]
fn mcp_channel_tools_path_under_workbench_root() {
    let sandbox = TestSandbox::new();
    let path = mcp_channel_tools_path().expect("mcp_channel_tools");
    assert_eq!(path, sandbox.workbench_root().join("mcp_channel_tools.json"));
}

#[test]
fn todo_tasks_path_under_workbench_root() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let path = todo_tasks_path().expect("todo_tasks");
    assert_eq!(path, wb.join("todo_tasks").join("todo_tasks.json"));
    assert!(
        !path.to_string_lossy().contains("plan_tasks"),
        "storage path must not retain plan_tasks segment: {path:?}"
    );
}

#[test]
fn knowledge_root_uses_settings() {
    let sandbox = TestSandbox::new();
    assert_eq!(
        knowledge_root().expect("knowledge_root"),
        sandbox.knowledge_root()
    );
}

#[test]
fn notes_root_is_workbench_root_notes() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let notes = notes_root().expect("notes_root");
    assert_eq!(notes, wb.join("notes"));
}

#[test]
fn ssot_paths_use_configured_workbench_root_not_cache() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let sediment = sediment_kb_dir().expect("sediment");
    let read_later = read_later_path().expect("read_later");
    let todo_tasks = todo_tasks_path().expect("todo_tasks");
    let message_center = message_center_path().expect("message_center");
    assert!(sediment.starts_with(&wb));
    assert!(read_later.starts_with(&wb));
    assert!(todo_tasks.starts_with(&wb));
    assert!(message_center.starts_with(&wb));
    let cache = cache_dir().expect("cache");
    assert!(!sediment.starts_with(&cache));
    assert!(!read_later.starts_with(&cache));
    assert!(!todo_tasks.starts_with(&cache));
    assert!(!message_center.starts_with(&cache));
}

#[test]
fn todo_tasks_v2_dir_and_index_under_workbench_root() {
    let _sandbox = TestSandbox::new();
    let wb = _sandbox.workbench_root();
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
    let wb = _sandbox.workbench_root();
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
fn todo_tasks_categories_path_under_todo_tasks_dir_not_sediment() {
    let sandbox = TestSandbox::new();
    let wb = sandbox.workbench_root();
    let dir = todo_tasks_dir().expect("todo_tasks_dir");
    let cats = todo_tasks_categories_path().expect("todo_tasks_categories_path");
    assert_eq!(cats, dir.join("categories.json"));
    assert_eq!(cats, wb.join("todo_tasks").join("categories.json"));
    let sediment_cats = sediment_kb_categories_path().expect("sediment cats");
    assert_ne!(
        cats, sediment_cats,
        "todo categories registry must not reuse sediment_kb path"
    );
}
