use super::*;

#[test]
fn todo_tasks_path_is_under_workbench_root() {
    with_todo_task_sandbox(|wb| {
        let path = paths::todo_tasks_path().expect("path");
        assert_eq!(path, wb.join("todo_tasks").join("todo_tasks.json"));
        let cache = paths::cache_dir().expect("cache");
        assert_ne!(path, cache.join("todo_tasks.json"));
        assert!(!path.starts_with(&cache));
    });
}

#[test]
fn first_write_creates_todo_tasks_directory() {
    with_todo_task_sandbox(|wb| {
        let todo_tasks_dir = wb.join("todo_tasks");
        assert!(!todo_tasks_dir.exists());
        let v = create_master_with_subs("预习：第三章", None);
        assert_eq!(v["_status"], 201);
        assert!(todo_tasks_dir.is_dir());
        assert!(todo_tasks_dir.join("index.json").is_file());
        assert!(todo_tasks_dir.join("tasks").is_dir());
    });
}

#[test]
fn todo_task_tests_do_not_touch_prod_todo_tasks_or_cache() {
    let before_wb = prod_todo_tasks_mtime();
    let before_cache = prod_cache_todo_tasks_mtime();
    with_todo_task_sandbox(|_| {
        create_master_with_subs("Isolation", Some(&["a", "b"]));
        list_all();
    });
    assert_eq!(before_wb, prod_todo_tasks_mtime());
    assert_eq!(before_cache, prod_cache_todo_tasks_mtime());
}

#[test]
fn todo_task_fixture_rejects_prod_todo_tasks_path() {
    let sandbox = TestSandbox::new();
    let prod_wb = sandbox.prod_workbench_root();
    let prod_todo_tasks = prod_wb.join("todo_tasks").join("todo_tasks.json");
    assert!(sandbox.assert_not_prod_path(&prod_todo_tasks).is_err());
    let prod_cache_plan = sandbox.prod_cache_dir().join("todo_tasks.json");
    assert!(sandbox.assert_not_prod_path(&prod_cache_plan).is_err());
}

#[test]
fn todo_task_paths_require_sandbox_isolation() {
    let sandbox = TestSandbox::new();
    let wb = paths::workbench_root().expect("wb");
    let plan_path = paths::todo_tasks_path().expect("todo_tasks");
    assert!(plan_path.starts_with(&wb));
    assert_ne!(
        plan_path,
        sandbox
            .prod_workbench_root()
            .join("todo_tasks")
            .join("todo_tasks.json")
    );
    assert_ne!(plan_path, sandbox.prod_cache_dir().join("todo_tasks.json"));
}

#[test]
fn create_persists_v2_layout_via_write_task_batch() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Persist", None);
        assert_eq!(created["_status"], 201);

        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        assert!(task_dir.join("sub_tasks.json").is_file());
        assert!(task_dir.join("todo.md").is_file());

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index["version"], 2);
        assert!(index["tasks"].get(master_id).is_some());
    });
}

#[test]
fn corrupt_storage_list_returns_explicit_error() {
    with_todo_task_sandbox(|wb| {
        let master_id = "task_corrupt_list";
        let todo_tasks_dir = wb.join("todo_tasks");
        fs::create_dir_all(todo_tasks_dir.join("tasks").join(master_id)).expect("mkdir");
        let index = serde_json::json!({
            "version": 2,
            "tasks": {
                master_id: {
                    "master_task_id": master_id,
                    "title": "Corrupt",
                    "status": "incomplete",
                    "created_at": "2026-07-08T00:00:00+00:00",
                    "task_dir": format!("tasks/{master_id}")
                }
            }
        });
        fs::write(
            todo_tasks_dir.join("index.json"),
            serde_json::to_string_pretty(&index).expect("serialize"),
        )
        .expect("write index");
        fs::write(
            todo_tasks_dir.join("tasks").join(master_id).join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let v = list_all();
        let listed = v.as_array().expect("array");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["master_task_id"], master_id);
        assert_eq!(listed[0]["migration_error"], true);
    });
}

#[test]
fn corrupt_storage_get_returns_explicit_error() {
    with_todo_task_sandbox(|wb| {
        let master_id = "task_corrupt_get";
        let todo_tasks_dir = wb.join("todo_tasks");
        fs::create_dir_all(todo_tasks_dir.join("tasks").join(master_id)).expect("mkdir");
        let index = serde_json::json!({
            "version": 2,
            "tasks": {
                master_id: {
                    "master_task_id": master_id,
                    "title": "Corrupt",
                    "status": "incomplete",
                    "created_at": "2026-07-08T00:00:00+00:00",
                    "task_dir": format!("tasks/{master_id}")
                }
            }
        });
        fs::write(
            todo_tasks_dir.join("index.json"),
            serde_json::to_string_pretty(&index).expect("serialize"),
        )
        .expect("write index");
        fs::write(
            todo_tasks_dir.join("tasks").join(master_id).join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let v = get_by_id(master_id);
        assert_eq!(v["master_task_id"], master_id);
        assert_eq!(v["migration_error"], true);
        assert!(v.get("_status").is_none());
    });
}

#[test]
fn bootstrap_deletes_wb_and_cache_v1_on_storage_read_entry() {
    with_todo_task_sandbox(|wb| {
        let wb_v1 = paths::todo_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_todo_tasks_v1_path().expect("cache v1 path");
        seed_v1_file(&wb_v1);
        seed_v1_file(&cache_v1);
        assert!(wb_v1.is_file());
        assert!(cache_v1.is_file());

        let v = list_all();
        assert!(v.as_array().is_some());

        assert!(!wb_v1.is_file(), "wb v1 should be deleted");
        assert!(!cache_v1.is_file(), "cache v1 should be deleted");
        assert_eq!(read_index_version(wb), 2);
    });
}

#[test]
fn bootstrap_deletes_v1_on_storage_write_entry() {
    with_todo_task_sandbox(|wb| {
        let wb_v1 = paths::todo_tasks_path().expect("wb v1 path");
        seed_v1_file(&wb_v1);
        assert!(wb_v1.is_file());

        let v = create_master_with_subs("Bootstrap via write", None);
        assert_eq!(v["_status"], 201);
        assert_eq!(read_index_version(wb), 2);
        assert!(!wb_v1.is_file(), "v2 create must not recreate v1 file");
    });
}

#[test]
fn bootstrap_repeat_is_idempotent() {
    with_todo_task_sandbox(|wb| {
        let wb_v1 = paths::todo_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_todo_tasks_v1_path().expect("cache v1 path");
        seed_v1_file(&wb_v1);
        seed_v1_file(&cache_v1);

        list_all();
        let index_path = wb.join("todo_tasks").join("index.json");
        let after_first = fs::read_to_string(&index_path).expect("read index after first");

        list_all();
        let after_second = fs::read_to_string(&index_path).expect("read index after second");

        assert!(!wb_v1.is_file());
        assert!(!cache_v1.is_file());
        assert_eq!(after_first, after_second);
        assert_eq!(read_index_version(wb), 2);
    });
}

#[test]
fn bootstrap_creates_todo_tasks_and_tasks_dirs() {
    with_todo_task_sandbox(|wb| {
        list_all();
        assert!(wb.join("todo_tasks").is_dir());
        assert!(wb.join("todo_tasks").join("tasks").is_dir());
        assert!(wb.join("todo_tasks").join("index.json").is_file());
    });
}

#[test]
fn bootstrap_deletes_only_wb_v1_when_cache_missing() {
    with_todo_task_sandbox(|wb| {
        let wb_v1 = paths::todo_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_todo_tasks_v1_path().expect("cache v1 path");
        seed_v1_file(&wb_v1);
        assert!(!cache_v1.is_file());

        list_all();

        assert!(!wb_v1.is_file());
        assert!(!cache_v1.is_file());
        assert_eq!(read_index_version(wb), 2);
    });
}

#[test]
fn bootstrap_deletes_only_cache_v1_when_wb_missing() {
    with_todo_task_sandbox(|wb| {
        let wb_v1 = paths::todo_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_todo_tasks_v1_path().expect("cache v1 path");
        seed_v1_file(&cache_v1);
        assert!(!wb_v1.is_file());

        list_all();

        assert!(!wb_v1.is_file());
        assert!(!cache_v1.is_file());
        assert_eq!(read_index_version(wb), 2);
    });
}

#[test]
fn bootstrap_preserves_valid_v2_index_tasks() {
    with_todo_task_sandbox(|wb| {
        let todo_tasks_dir = wb.join("todo_tasks");
        fs::create_dir_all(todo_tasks_dir.join("tasks")).expect("mkdir tasks");
        let index = serde_json::json!({
            "version": 2,
            "tasks": {
                "task_keepme": {
                    "master_task_id": "task_keepme",
                    "title": "Keep me",
                    "status": "incomplete",
                    "created_at": "2026-07-08T00:00:00+00:00",
                    "task_dir": "tasks/task_keepme"
                }
            }
        });
        fs::write(
            todo_tasks_dir.join("index.json"),
            serde_json::to_string_pretty(&index).expect("serialize index"),
        )
        .expect("write index");
        let sub_tasks = serde_json::json!({
            "sub_tasks": [{
                "sub_task_id": "task_keepme_sub_01",
                "title": "Keep me",
                "status": "incomplete",
                "implicit": true,
                "linked_archive_ids": []
            }]
        });
        fs::create_dir_all(todo_tasks_dir.join("tasks").join("task_keepme")).expect("mkdir task");
        fs::write(
            todo_tasks_dir.join("tasks").join("task_keepme").join("sub_tasks.json"),
            serde_json::to_string_pretty(&sub_tasks).expect("serialize subs"),
        )
        .expect("write sub_tasks");
        fs::write(
            todo_tasks_dir.join("tasks").join("task_keepme").join("todo.md"),
            "",
        )
        .expect("write plan.md");

        list_all();

        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(todo_tasks_dir.join("index.json")).unwrap())
                .expect("parse index");
        assert_eq!(parsed["version"], 2);
        assert!(parsed["tasks"].get("task_keepme").is_some());
    });
}

#[test]
fn bootstrap_rewrites_wrong_version_index() {
    with_todo_task_sandbox(|wb| {
        let todo_tasks_dir = wb.join("todo_tasks");
        fs::create_dir_all(todo_tasks_dir.join("tasks")).expect("mkdir tasks");
        fs::write(
            todo_tasks_dir.join("index.json"),
            r#"{"version":1,"tasks":{"task_old":{"master_task_id":"task_old"}}}"#,
        )
        .expect("write v1 index");

        list_all();

        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(todo_tasks_dir.join("index.json")).unwrap())
                .expect("parse index");
        assert_eq!(parsed["version"], 2);
        assert_eq!(
            parsed["tasks"].as_object().map(|m| m.len()).unwrap_or(0),
            0
        );
    });
}

#[test]
fn bootstrap_rewrites_corrupt_index() {
    with_todo_task_sandbox(|wb| {
        let todo_tasks_dir = wb.join("todo_tasks");
        fs::create_dir_all(todo_tasks_dir.join("tasks")).expect("mkdir tasks");
        fs::write(todo_tasks_dir.join("index.json"), "{not json").expect("write corrupt index");

        list_all();

        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(todo_tasks_dir.join("index.json")).unwrap())
                .expect("parse index");
        assert_eq!(parsed["version"], 2);
        assert_eq!(
            parsed["tasks"].as_object().map(|m| m.len()).unwrap_or(0),
            0
        );
    });
}

#[test]
fn write_task_batch_creates_v2_files_and_index_entry() {
    with_todo_task_sandbox(|wb| {
        let master_id = "task_batch001";
        test_run_write_task_batch(
            master_id,
            &sample_index_entry(master_id),
            &sample_sub_tasks(master_id),
            "",
        )
        .expect("batch write");

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        assert!(task_dir.join("sub_tasks.json").is_file());
        assert!(task_dir.join("todo.md").is_file());
        assert_eq!(fs::read_to_string(task_dir.join("todo.md")).unwrap(), "");

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index["version"], 2);
        assert!(index["tasks"].get(master_id).is_some());
    });
}

#[test]
fn write_task_batch_sub_tasks_failure_removes_task_dir() {
    with_todo_task_sandbox(|wb| {
        let master_id = "task_batch002";
        test_set_fail_batch_sub_tasks(true);
        assert!(test_run_write_task_batch(
            master_id,
            &sample_index_entry(master_id),
            &sample_sub_tasks(master_id),
            "",
        )
        .is_err());

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        assert!(!task_dir.exists());
        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert!(index["tasks"].get(master_id).is_none());
    });
}

#[test]
fn write_task_batch_todo_md_failure_removes_task_dir() {
    with_todo_task_sandbox(|wb| {
        let master_id = "task_batch003";
        test_set_fail_batch_todo_md(true);
        assert!(test_run_write_task_batch(
            master_id,
            &sample_index_entry(master_id),
            &sample_sub_tasks(master_id),
            "body",
        )
        .is_err());

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        assert!(!task_dir.exists());
    });
}

#[test]
fn write_task_batch_index_failure_restores_snapshot_and_leaves_orphan() {
    with_todo_task_sandbox(|wb| {
        let existing_id = "task_existing";
        let existing_entry = sample_index_entry(existing_id);
        test_run_write_task_batch(
            existing_id,
            &existing_entry,
            &sample_sub_tasks(existing_id),
            "keep",
        )
        .expect("seed existing");

        let index_path = wb.join("todo_tasks").join("index.json");
        let snapshot_before = fs::read_to_string(&index_path).unwrap();

        let master_id = "task_batch004";
        test_set_fail_batch_index(true);
        assert!(test_run_write_task_batch(
            master_id,
            &sample_index_entry(master_id),
            &sample_sub_tasks(master_id),
            "orphan",
        )
        .is_err());

        let snapshot_after = fs::read_to_string(&index_path).unwrap();
        assert_eq!(snapshot_before, snapshot_after);

        let orphan_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        assert!(orphan_dir.join("sub_tasks.json").is_file());
        assert!(orphan_dir.join("todo.md").is_file());

        let index: serde_json::Value = serde_json::from_str(&snapshot_after).unwrap();
        assert!(index["tasks"].get(master_id).is_none());
        assert!(index["tasks"].get(existing_id).is_some());
    });
}

#[test]
fn injection_flags_reset_before_each_sandbox_test() {
    with_todo_task_sandbox(|_| {
        test_set_fail_batch_sub_tasks(true);
        let created = create_master_with_subs("Flag reset", None);
        assert_eq!(created["_status"], 500);
    });
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("After reset", None);
        assert_eq!(created["_status"], 201);
    });
}

#[test]
fn create_batch_failure_leaves_no_partial_commit() {
    with_todo_task_sandbox(|wb| {
        test_set_fail_batch_index(true);
        let v = create_master_with_subs("Fail batch", None);
        assert_eq!(v["_status"], 500);

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index["tasks"].as_object().map(|m| m.len()).unwrap_or(0), 0);

        let list = list_all();
        let list = list.as_array().expect("array");
        assert!(list.is_empty());
    });
}
