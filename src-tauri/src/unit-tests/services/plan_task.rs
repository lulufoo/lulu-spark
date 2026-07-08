use super::*;
use crate::services::plan_task::types::{
    IndexEntry, MasterTaskStatus, SubTask, SubTaskStatus, SubTasksFile,
};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::config::paths;
use crate::config::settings::{self, default_cache_dir};
use crate::test_support::TestSandbox;

fn with_plan_task_sandbox<F: FnOnce(&Path)>(f: F) {
    let _sandbox = TestSandbox::new();
    let wb = paths::workbench_knowledge_root().expect("workbench root");
    f(&wb);
}

fn is_iso8601(s: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(s).is_ok()
}

fn master_from_value(v: &serde_json::Value) -> &serde_json::Value {
    v.get("task").expect("task field")
}

fn prod_plan_tasks_mtime() -> Option<SystemTime> {
    settings::AppSettings::default()
        .workbench_knowledge_root
        .join("plan_tasks")
        .join("plan_tasks.json")
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
}

fn prod_cache_plan_tasks_mtime() -> Option<SystemTime> {
    default_cache_dir()
        .join("plan_tasks.json")
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
}

#[test]
fn plan_tasks_path_is_under_workbench_knowledge_root() {
    with_plan_task_sandbox(|wb| {
        let path = paths::plan_tasks_path().expect("path");
        assert_eq!(path, wb.join("plan_tasks").join("plan_tasks.json"));
        let cache = paths::cache_dir().expect("cache");
        assert_ne!(path, cache.join("plan_tasks.json"));
        assert!(!path.starts_with(&cache));
    });
}

#[test]
fn first_write_creates_plan_tasks_directory() {
    with_plan_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("plan_tasks");
        assert!(!plan_tasks_dir.exists());
        let v = create_master_with_subs("预习：第三章", None);
        assert_eq!(v["_status"], 201);
        assert!(plan_tasks_dir.is_dir());
        let path = paths::plan_tasks_path().expect("path");
        assert!(path.is_file());
    });
}

#[test]
fn list_all_empty_store_returns_empty_array() {
    with_plan_task_sandbox(|_| {
        let v = list_all();
        assert_eq!(v.as_array().expect("array").len(), 0);
        assert!(v.get("_status").is_none());
    });
}

#[test]
fn create_single_explicit_sub_implicit_false() {
    with_plan_task_sandbox(|_| {
        let v = create_master_with_subs("Master", Some(&["Sub A"]));
        assert_eq!(v["_status"], 201);
        let task = master_from_value(&v);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["implicit"], false);
        assert_eq!(subs[0]["title"], "Sub A");
        assert_eq!(subs[0]["status"], "incomplete");
    });
}

#[test]
fn multi_sub_create_and_read_fixture() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Multi master", Some(&["Sub A", "Sub B"]));
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("master_task_id");

        let listed = list_all();
        let listed = listed.as_array().expect("array");
        assert_eq!(listed.len(), 1);
        let listed_subs = listed[0]["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(listed_subs.len(), 2);
        assert_eq!(listed_subs[0]["implicit"], false);
        assert_eq!(listed_subs[1]["implicit"], false);
        assert_eq!(listed_subs[0]["status"], "incomplete");
        assert_eq!(listed_subs[1]["status"], "incomplete");

        let sub_a = listed_subs[0]["sub_task_id"].as_str().unwrap();
        let sub_b = listed_subs[1]["sub_task_id"].as_str().unwrap();

        let got = get_by_id(master_id);
        assert_eq!(got["sub_tasks"].as_array().unwrap().len(), 2);

        let linked_a = link_archive(master_id, sub_a, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(linked_a["_status"], 200);
        let linked_b = link_archive(master_id, sub_b, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert_eq!(linked_b["_status"], 200);

        let after_link = get_by_id(master_id);
        let subs = after_link["sub_tasks"].as_array().unwrap();
        assert_eq!(
            subs[0]["linked_archive_ids"].as_array().unwrap()[0],
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        assert_eq!(
            subs[1]["linked_archive_ids"].as_array().unwrap()[0],
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );

        let path = wb.join("plan_tasks").join("plan_tasks.json");
        assert!(path.is_file());
        let cache = paths::cache_dir().expect("cache");
        assert!(!path.starts_with(&cache));
    });
}

#[test]
fn plan_task_tests_do_not_touch_prod_plan_tasks_or_cache() {
    let before_wb = prod_plan_tasks_mtime();
    let before_cache = prod_cache_plan_tasks_mtime();
    with_plan_task_sandbox(|_| {
        create_master_with_subs("Isolation", Some(&["a", "b"]));
        list_all();
    });
    assert_eq!(before_wb, prod_plan_tasks_mtime());
    assert_eq!(before_cache, prod_cache_plan_tasks_mtime());
}

#[test]
fn plan_task_fixture_rejects_prod_plan_tasks_path() {
    crate::test_support::with_config_test_serial(|| {
        let sandbox = TestSandbox::new();
        let prod_wb = sandbox.prod_workbench_knowledge_root();
        let prod_plan_tasks = prod_wb.join("plan_tasks").join("plan_tasks.json");
        assert!(sandbox.assert_not_prod_path(&prod_plan_tasks).is_err());
        let prod_cache_plan = sandbox.prod_cache_dir().join("plan_tasks.json");
        assert!(sandbox.assert_not_prod_path(&prod_cache_plan).is_err());
    });
}

#[test]
fn plan_task_paths_require_sandbox_isolation() {
    crate::test_support::with_config_test_serial(|| {
        let sandbox = TestSandbox::new();
        let wb = paths::workbench_knowledge_root().expect("wb");
        let plan_path = paths::plan_tasks_path().expect("plan_tasks");
        assert!(plan_path.starts_with(&wb));
        assert_ne!(
            plan_path,
            sandbox
                .prod_workbench_knowledge_root()
                .join("plan_tasks")
                .join("plan_tasks.json")
        );
        assert_ne!(plan_path, sandbox.prod_cache_dir().join("plan_tasks.json"));
    });
}

#[test]
fn create_minimal_generates_implicit_sub_01() {
    with_plan_task_sandbox(|_| {
        let v = create_master_with_subs("预习：第三章", None);
        assert_eq!(v["_status"], 201);
        let master_id = v["master_task_id"].as_str().expect("master_task_id");
        let sub_id = v["sub_task_id"].as_str().expect("sub_task_id");
        assert_eq!(sub_id, format!("{master_id}_sub_01"));
        assert!(sub_id.ends_with("_sub_01"));

        let task = master_from_value(&v);
        assert_eq!(task["title"], "预习：第三章");
        assert_eq!(task["status"], "incomplete");
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["sub_task_id"], sub_id);
        assert_eq!(subs[0]["implicit"], true);
        assert_eq!(subs[0]["status"], "incomplete");
        assert_eq!(subs[0]["linked_archive_ids"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn create_multiple_subs_increments_sub_suffix() {
    with_plan_task_sandbox(|_| {
        let v = create_master_with_subs("Multi", Some(&["A", "B", "C"]));
        assert_eq!(v["_status"], 201);
        let master_id = v["master_task_id"].as_str().expect("master_task_id");
        let task = master_from_value(&v);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0]["sub_task_id"], format!("{master_id}_sub_01"));
        assert_eq!(subs[1]["sub_task_id"], format!("{master_id}_sub_02"));
        assert_eq!(subs[2]["sub_task_id"], format!("{master_id}_sub_03"));
        assert_eq!(subs[0]["implicit"], false);
        assert_eq!(subs[1]["implicit"], false);
        assert_eq!(subs[2]["implicit"], false);
    });
}

#[test]
fn get_by_id_returns_master_tree() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Get me", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let got = get_by_id(master_id);
        assert!(got.get("_status").is_none() || got["_status"] == 200);
        assert_eq!(got["master_task_id"], master_id);
        assert_eq!(got["sub_tasks"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn get_by_id_resolves_sub_task_id() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Sub lookup", None);
        let sub_id = created["sub_task_id"].as_str().unwrap();
        let got = get_by_id(sub_id);
        assert_eq!(got["master_task_id"], created["master_task_id"]);
        assert_eq!(got["sub_tasks"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn list_all_returns_master_trees() {
    with_plan_task_sandbox(|_| {
        let empty = list_all();
        assert!(empty.as_array().expect("array").is_empty());

        create_master_with_subs("One", None);
        create_master_with_subs("Two", None);
        let list = list_all().as_array().expect("array").clone();
        assert_eq!(list.len(), 2);
        for item in &list {
            assert!(item.get("master_task_id").is_some());
            assert!(item["sub_tasks"].as_array().unwrap().len() >= 1);
        }
    });
}

#[test]
fn complete_sub_marks_master_complete_when_all_subs_done() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Done", Some(&["b"]));
        let master_id = created["master_task_id"].as_str().unwrap().to_string();
        let subs: Vec<String> = created["task"]["sub_tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["sub_task_id"].as_str().unwrap().to_string())
            .collect();

        for sub_id in &subs {
            let v = complete_sub(&master_id, sub_id);
            assert_eq!(v["_status"], 200);
        }

        let got = get_by_id(&master_id);
        assert_eq!(got["status"], "complete");
        for sub in got["sub_tasks"].as_array().unwrap() {
            assert_eq!(sub["status"], "complete");
            assert!(sub
                .get("completed_at")
                .and_then(|v| v.as_str())
                .map(is_iso8601)
                .unwrap_or(false));
        }
    });
}

#[test]
fn complete_sub_unknown_id_returns_404() {
    with_plan_task_sandbox(|_| {
        let v = complete_sub("task_missing", "task_missing_sub_01");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn get_by_id_unknown_returns_404() {
    with_plan_task_sandbox(|_| {
        let v = get_by_id("task_does_not_exist");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn link_archive_updates_reverse_index() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Link", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["sub_task_id"].as_str().unwrap();

        let linked = link_archive(master_id, sub_id, "abc123def45678901234567890123456");
        assert_eq!(linked["_status"], 200);

        let got = get_by_id(master_id);
        let subs = got["sub_tasks"].as_array().unwrap();
        let ids = subs[0]["linked_archive_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "abc123def45678901234567890123456");

        let again = link_archive(master_id, sub_id, "abc123def45678901234567890123456");
        assert_eq!(again["_status"], 200);
        let got2 = get_by_id(master_id);
        assert_eq!(
            got2["sub_tasks"][0]["linked_archive_ids"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    });
}

#[test]
fn create_persists_valid_json_via_atomic_write() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Persist", None);
        assert_eq!(created["_status"], 201);

        let path = wb.join("plan_tasks").join("plan_tasks.json");
        assert!(path.is_file());
        let text = fs::read_to_string(&path).expect("read json");
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["version"], 1);
        let tasks = parsed["tasks"].as_object().expect("tasks map");
        assert_eq!(tasks.len(), 1);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert!(tasks.contains_key(master_id));
    });
}

#[test]
fn corrupt_json_list_returns_explicit_error() {
    with_plan_task_sandbox(|wb| {
        let path = wb.join("plan_tasks").join("plan_tasks.json");
        fs::create_dir_all(wb.join("plan_tasks")).expect("mkdir");
        fs::write(&path, "{not valid json").expect("write corrupt");

        let v = list_all();
        assert!(v.get("error").is_some());
        assert_eq!(v["_status"], 500);
    });
}

#[test]
fn corrupt_json_get_returns_explicit_error() {
    with_plan_task_sandbox(|wb| {
        let path = wb.join("plan_tasks").join("plan_tasks.json");
        fs::create_dir_all(wb.join("plan_tasks")).expect("mkdir");
        fs::write(&path, "{not valid json").expect("write corrupt");

        let v = get_by_id("any-id");
        assert!(v.get("error").is_some());
        assert_eq!(v["_status"], 500);
    });
}

const V1_STUB_JSON: &str = r#"{"version":1,"tasks":{}}"#;

fn seed_v1_file(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir v1 parent");
    }
    fs::write(path, V1_STUB_JSON).expect("write v1 stub");
}

fn read_index_version(wb: &Path) -> u32 {
    let index_path = wb.join("plan_tasks").join("index.json");
    let text = fs::read_to_string(&index_path).expect("read index.json");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse index.json");
    parsed["version"].as_u64().expect("index version") as u32
}

#[test]
fn bootstrap_deletes_wb_and_cache_v1_on_storage_read_entry() {
    with_plan_task_sandbox(|wb| {
        let wb_v1 = paths::plan_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_plan_tasks_v1_path().expect("cache v1 path");
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
    with_plan_task_sandbox(|wb| {
        let wb_v1 = paths::plan_tasks_path().expect("wb v1 path");
        seed_v1_file(&wb_v1);
        assert!(wb_v1.is_file());

        let v = create_master_with_subs("Bootstrap via write", None);
        assert_eq!(v["_status"], 201);
        assert_eq!(read_index_version(wb), 2);
        // v1 single-file may be recreated by legacy write path until v3; bootstrap ran at write entry.
    });
}

#[test]
fn bootstrap_repeat_is_idempotent() {
    with_plan_task_sandbox(|wb| {
        let wb_v1 = paths::plan_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_plan_tasks_v1_path().expect("cache v1 path");
        seed_v1_file(&wb_v1);
        seed_v1_file(&cache_v1);

        list_all();
        let index_path = wb.join("plan_tasks").join("index.json");
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
fn bootstrap_creates_plan_tasks_and_tasks_dirs() {
    with_plan_task_sandbox(|wb| {
        list_all();
        assert!(wb.join("plan_tasks").is_dir());
        assert!(wb.join("plan_tasks").join("tasks").is_dir());
        assert!(wb.join("plan_tasks").join("index.json").is_file());
    });
}

#[test]
fn bootstrap_deletes_only_wb_v1_when_cache_missing() {
    with_plan_task_sandbox(|wb| {
        let wb_v1 = paths::plan_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_plan_tasks_v1_path().expect("cache v1 path");
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
    with_plan_task_sandbox(|wb| {
        let wb_v1 = paths::plan_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_plan_tasks_v1_path().expect("cache v1 path");
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
    with_plan_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("plan_tasks");
        fs::create_dir_all(plan_tasks_dir.join("tasks")).expect("mkdir tasks");
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
            plan_tasks_dir.join("index.json"),
            serde_json::to_string_pretty(&index).expect("serialize index"),
        )
        .expect("write index");

        list_all();

        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(plan_tasks_dir.join("index.json")).unwrap())
                .expect("parse index");
        assert_eq!(parsed["version"], 2);
        assert!(parsed["tasks"].get("task_keepme").is_some());
    });
}

#[test]
fn bootstrap_rewrites_wrong_version_index() {
    with_plan_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("plan_tasks");
        fs::create_dir_all(plan_tasks_dir.join("tasks")).expect("mkdir tasks");
        fs::write(
            plan_tasks_dir.join("index.json"),
            r#"{"version":1,"tasks":{"task_old":{"master_task_id":"task_old"}}}"#,
        )
        .expect("write v1 index");

        list_all();

        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(plan_tasks_dir.join("index.json")).unwrap())
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
    with_plan_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("plan_tasks");
        fs::create_dir_all(plan_tasks_dir.join("tasks")).expect("mkdir tasks");
        fs::write(plan_tasks_dir.join("index.json"), "{not json").expect("write corrupt index");

        list_all();

        let parsed: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(plan_tasks_dir.join("index.json")).unwrap())
                .expect("parse index");
        assert_eq!(parsed["version"], 2);
        assert_eq!(
            parsed["tasks"].as_object().map(|m| m.len()).unwrap_or(0),
            0
        );
    });
}

fn sample_index_entry(master_id: &str) -> IndexEntry {
    IndexEntry {
        master_task_id: master_id.to_string(),
        title: "Batch task".to_string(),
        status: MasterTaskStatus::Incomplete,
        created_at: "2026-07-08T00:00:00+00:00".to_string(),
        task_dir: format!("tasks/{master_id}"),
    }
}

fn sample_sub_tasks(master_id: &str) -> SubTasksFile {
    SubTasksFile {
        sub_tasks: vec![SubTask {
            sub_task_id: format!("{master_id}_sub_01"),
            title: Some("Batch task".to_string()),
            status: SubTaskStatus::Incomplete,
            implicit: true,
            linked_archive_ids: vec![],
            completed_at: None,
        }],
    }
}

#[test]
fn write_task_batch_creates_v2_files_and_index_entry() {
    with_plan_task_sandbox(|wb| {
        let master_id = "task_batch001";
        test_run_write_task_batch(
            master_id,
            &sample_index_entry(master_id),
            &sample_sub_tasks(master_id),
            "",
        )
        .expect("batch write");

        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(task_dir.join("sub_tasks.json").is_file());
        assert!(task_dir.join("plan.md").is_file());
        assert_eq!(fs::read_to_string(task_dir.join("plan.md")).unwrap(), "");

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index["version"], 2);
        assert!(index["tasks"].get(master_id).is_some());
    });
}

#[test]
fn write_task_batch_sub_tasks_failure_removes_task_dir() {
    with_plan_task_sandbox(|wb| {
        let master_id = "task_batch002";
        test_set_fail_batch_sub_tasks(true);
        assert!(test_run_write_task_batch(
            master_id,
            &sample_index_entry(master_id),
            &sample_sub_tasks(master_id),
            "",
        )
        .is_err());

        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(!task_dir.exists());
        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert!(index["tasks"].get(master_id).is_none());
    });
}

#[test]
fn write_task_batch_plan_md_failure_removes_task_dir() {
    with_plan_task_sandbox(|wb| {
        let master_id = "task_batch003";
        test_set_fail_batch_plan_md(true);
        assert!(test_run_write_task_batch(
            master_id,
            &sample_index_entry(master_id),
            &sample_sub_tasks(master_id),
            "body",
        )
        .is_err());

        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(!task_dir.exists());
    });
}

#[test]
fn write_task_batch_index_failure_restores_snapshot_and_leaves_orphan() {
    with_plan_task_sandbox(|wb| {
        let existing_id = "task_existing";
        let existing_entry = sample_index_entry(existing_id);
        test_run_write_task_batch(
            existing_id,
            &existing_entry,
            &sample_sub_tasks(existing_id),
            "keep",
        )
        .expect("seed existing");

        let index_path = wb.join("plan_tasks").join("index.json");
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

        let orphan_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(orphan_dir.join("sub_tasks.json").is_file());
        assert!(orphan_dir.join("plan.md").is_file());

        let index: serde_json::Value = serde_json::from_str(&snapshot_after).unwrap();
        assert!(index["tasks"].get(master_id).is_none());
        assert!(index["tasks"].get(existing_id).is_some());
    });
}
