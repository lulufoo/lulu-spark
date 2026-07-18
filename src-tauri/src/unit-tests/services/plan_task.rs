use super::*;
use crate::services::plan_task::types::{
    AttachmentsFile, IndexEntry, MasterTaskStatus, SubTask, SubTaskStatus, SubTasksFile,
};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::config::paths;
use crate::config::settings::{self, default_cache_dir};
use crate::services::plan_task::test_reset_all_injection_flags;
use crate::test_support::TestSandbox;

fn with_plan_task_sandbox<F: FnOnce(&Path)>(f: F) {
    let _sandbox = TestSandbox::new();
    test_reset_all_injection_flags();
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
        assert!(plan_tasks_dir.join("index.json").is_file());
        assert!(plan_tasks_dir.join("tasks").is_dir());
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

        let path = wb.join("plan_tasks").join("index.json");
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
    let sandbox = TestSandbox::new();
    let prod_wb = sandbox.prod_workbench_knowledge_root();
    let prod_plan_tasks = prod_wb.join("plan_tasks").join("plan_tasks.json");
    assert!(sandbox.assert_not_prod_path(&prod_plan_tasks).is_err());
    let prod_cache_plan = sandbox.prod_cache_dir().join("plan_tasks.json");
    assert!(sandbox.assert_not_prod_path(&prod_cache_plan).is_err());
}

#[test]
fn plan_task_paths_require_sandbox_isolation() {
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
}

#[test]
fn create_without_sub_titles_yields_empty_sub_tasks() {
    with_plan_task_sandbox(|_| {
        let v = create_master_with_subs("预习：第三章", None);
        assert_eq!(v["_status"], 201);
        assert!(v.get("sub_task_id").is_none());

        let task = master_from_value(&v);
        assert_eq!(task["title"], "预习：第三章");
        assert_eq!(task["status"], "incomplete");
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
    });
}

#[test]
fn create_with_empty_sub_titles_slice_yields_empty_sub_tasks() {
    with_plan_task_sandbox(|_| {
        let empty: &[&str] = &[];
        let v = create_master_with_subs("Empty slice", Some(empty));
        assert_eq!(v["_status"], 201);
        let subs = master_from_value(&v)["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
        assert_eq!(master_from_value(&v)["status"], "incomplete");
    });
}

#[test]
fn recompute_master_status_empty_sub_tasks_stays_incomplete() {
    with_plan_task_sandbox(|_| {
        let master_id = "task_empty_subs";
        let mut entry = sample_index_entry(master_id);
        entry.status = MasterTaskStatus::Complete;
        test_run_write_task_batch(
            master_id,
            &entry,
            &SubTasksFile { sub_tasks: vec![] },
            "",
        )
        .expect("batch write");

        let got = get_by_id(master_id);
        assert_eq!(got["status"], "incomplete");
        assert_eq!(got["sub_tasks"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn recompute_master_status_all_complete_marks_master_complete() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("All done", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        for sub in created["task"]["sub_tasks"].as_array().unwrap() {
            let sub_id = sub["sub_task_id"].as_str().unwrap();
            let v = complete_sub(master_id, sub_id);
            assert_eq!(v["_status"], 200);
        }
        let got = get_by_id(master_id);
        assert_eq!(got["status"], "complete");
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
        let created = create_master_with_subs("Get me", Some(&["Sub"]));
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
        let created = create_master_with_subs("Sub lookup", Some(&["Sub"]));
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
        create_master_with_subs("Two", Some(&["Sub"]));
        let list = list_all().as_array().expect("array").clone();
        assert_eq!(list.len(), 2);
        for item in &list {
            assert!(item.get("master_task_id").is_some());
            assert!(item["sub_tasks"].as_array().is_some());
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
        let created = create_master_with_subs("Link", Some(&["Sub"]));
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
fn create_persists_v2_layout_via_write_task_batch() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Persist", None);
        assert_eq!(created["_status"], 201);

        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(task_dir.join("sub_tasks.json").is_file());
        assert!(task_dir.join("plan.md").is_file());

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index["version"], 2);
        assert!(index["tasks"].get(master_id).is_some());
    });
}

#[test]
fn corrupt_storage_list_returns_explicit_error() {
    with_plan_task_sandbox(|wb| {
        let master_id = "task_corrupt_list";
        let plan_tasks_dir = wb.join("plan_tasks");
        fs::create_dir_all(plan_tasks_dir.join("tasks").join(master_id)).expect("mkdir");
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
            plan_tasks_dir.join("index.json"),
            serde_json::to_string_pretty(&index).expect("serialize"),
        )
        .expect("write index");
        fs::write(
            plan_tasks_dir.join("tasks").join(master_id).join("sub_tasks.json"),
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
    with_plan_task_sandbox(|wb| {
        let master_id = "task_corrupt_get";
        let plan_tasks_dir = wb.join("plan_tasks");
        fs::create_dir_all(plan_tasks_dir.join("tasks").join(master_id)).expect("mkdir");
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
            plan_tasks_dir.join("index.json"),
            serde_json::to_string_pretty(&index).expect("serialize"),
        )
        .expect("write index");
        fs::write(
            plan_tasks_dir.join("tasks").join(master_id).join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let v = get_by_id(master_id);
        assert_eq!(v["master_task_id"], master_id);
        assert_eq!(v["migration_error"], true);
        assert!(v.get("_status").is_none());
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
        assert!(!wb_v1.is_file(), "v2 create must not recreate v1 file");
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
        let sub_tasks = serde_json::json!({
            "sub_tasks": [{
                "sub_task_id": "task_keepme_sub_01",
                "title": "Keep me",
                "status": "incomplete",
                "implicit": true,
                "linked_archive_ids": []
            }]
        });
        fs::create_dir_all(plan_tasks_dir.join("tasks").join("task_keepme")).expect("mkdir task");
        fs::write(
            plan_tasks_dir.join("tasks").join("task_keepme").join("sub_tasks.json"),
            serde_json::to_string_pretty(&sub_tasks).expect("serialize subs"),
        )
        .expect("write sub_tasks");
        fs::write(
            plan_tasks_dir.join("tasks").join("task_keepme").join("plan.md"),
            "",
        )
        .expect("write plan.md");

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

#[test]
fn add_sub_appends_incomplete_sub_and_recomputes_master_incomplete() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Add sub", Some(&["A"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        complete_sub(master_id, created["sub_task_id"].as_str().unwrap());
        let got_before = get_by_id(master_id);
        assert_eq!(got_before["status"], "complete");

        let added = add_sub(master_id, "B");
        assert_eq!(added["_status"], 201);
        assert!(added.get("sub_task_id").is_some());
        let task = master_from_value(&added);
        assert_eq!(task["status"], "incomplete");
        let subs = task["sub_tasks"].as_array().unwrap();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[1]["status"], "incomplete");
        assert_eq!(subs[1]["implicit"], false);
    });
}

#[test]
fn complete_sub_partial_multi_sub_keeps_master_incomplete() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Partial", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_a = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        let v = complete_sub(master_id, sub_a);
        assert_eq!(v["_status"], 200);
        assert_eq!(master_from_value(&v)["status"], "incomplete");
    });
}

#[test]
fn link_archive_does_not_change_sub_or_master_status() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Link status", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_a = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        complete_sub(master_id, sub_a);
        let before = get_by_id(master_id);
        let linked = link_archive(master_id, sub_a, "archive_id_123456789012345678901234");
        assert_eq!(linked["_status"], 200);
        let after = master_from_value(&linked);
        assert_eq!(after["status"], before["status"]);
        assert_eq!(after["sub_tasks"][0]["status"], before["sub_tasks"][0]["status"]);
    });
}

#[test]
fn delete_sub_recomputes_master_and_allows_delete_to_empty() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Delete sub", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_a = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        complete_sub(master_id, sub_a);

        let deleted = delete_sub(master_id, sub_a);
        assert_eq!(deleted["_status"], 200);
        assert_eq!(master_from_value(&deleted)["status"], "incomplete");

        let last_sub = get_by_id(master_id)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap()
            .to_string();
        let deleted_last = delete_sub(master_id, &last_sub);
        assert_eq!(deleted_last["_status"], 200);
        assert_eq!(master_from_value(&deleted_last)["status"], "incomplete");
        assert!(master_from_value(&deleted_last)["sub_tasks"]
            .as_array()
            .unwrap()
            .is_empty());

        let got = get_by_id(master_id);
        assert_eq!(got["status"], "incomplete");
        assert!(got["sub_tasks"].as_array().unwrap().is_empty());
    });
}

#[test]
fn complete_sub_transitions_incomplete_to_complete_terminal() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Complete transition", Some(&["A"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        assert_eq!(created["task"]["sub_tasks"][0]["status"], "incomplete");

        let v = complete_sub(master_id, sub_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(master_from_value(&v)["sub_tasks"][0]["status"], "complete");
        assert!(master_from_value(&v)["sub_tasks"][0]
            .get("completed_at")
            .and_then(|v| v.as_str())
            .map(is_iso8601)
            .unwrap_or(false));
    });
}

#[test]
fn abandon_sub_transitions_incomplete_to_abandoned_terminal() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Abandon transition", Some(&["A"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        assert_eq!(created["task"]["sub_tasks"][0]["status"], "incomplete");

        let v = abandon_sub(master_id, sub_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(master_from_value(&v)["sub_tasks"][0]["status"], "abandoned");
        assert!(master_from_value(&v)["sub_tasks"][0]
            .get("completed_at")
            .is_none());
    });
}

#[test]
fn injection_flags_reset_before_each_sandbox_test() {
    with_plan_task_sandbox(|_| {
        test_set_fail_batch_sub_tasks(true);
        let created = create_master_with_subs("Flag reset", None);
        assert_eq!(created["_status"], 500);
    });
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("After reset", None);
        assert_eq!(created["_status"], 201);
    });
}

#[test]
fn abandon_sub_marks_sub_abandoned_and_recomputes_master_incomplete() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Abandon", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_a = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();

        let v = abandon_sub(master_id, sub_a);
        assert_eq!(v["_status"], 200);
        let task = master_from_value(&v);
        assert_eq!(task["status"], "incomplete");
        assert_eq!(task["sub_tasks"][0]["status"], "abandoned");
        assert_eq!(task["sub_tasks"][1]["status"], "incomplete");
    });
}

#[test]
fn abandon_sub_all_abandoned_keeps_master_incomplete() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("All abandoned", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        for sub in created["task"]["sub_tasks"].as_array().unwrap() {
            let sub_id = sub["sub_task_id"].as_str().unwrap();
            let v = abandon_sub(master_id, sub_id);
            assert_eq!(v["_status"], 200);
        }
        let got = get_by_id(master_id);
        assert_eq!(got["status"], "incomplete");
        for sub in got["sub_tasks"].as_array().unwrap() {
            assert_eq!(sub["status"], "abandoned");
        }
    });
}

#[test]
fn complete_and_abandoned_mix_keeps_master_incomplete() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Mixed terminal", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_a = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        let sub_b = created["task"]["sub_tasks"][1]["sub_task_id"]
            .as_str()
            .unwrap();

        assert_eq!(complete_sub(master_id, sub_a)["_status"], 200);
        assert_eq!(abandon_sub(master_id, sub_b)["_status"], 200);

        let got = get_by_id(master_id);
        assert_eq!(got["status"], "incomplete");
        assert_eq!(got["sub_tasks"][0]["status"], "complete");
        assert_eq!(got["sub_tasks"][1]["status"], "abandoned");
    });
}

#[test]
fn complete_sub_rejects_terminal_sub_returns_409() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Terminal guard", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_a = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        let sub_b = created["task"]["sub_tasks"][1]["sub_task_id"]
            .as_str()
            .unwrap();

        assert_eq!(complete_sub(master_id, sub_a)["_status"], 200);
        let again = complete_sub(master_id, sub_a);
        assert_eq!(again["_status"], 409);

        assert_eq!(abandon_sub(master_id, sub_b)["_status"], 200);
        let on_abandoned = complete_sub(master_id, sub_b);
        assert_eq!(on_abandoned["_status"], 409);
    });
}

#[test]
fn abandon_sub_rejects_terminal_sub_returns_409() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Abandon guard", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_a = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();
        let sub_b = created["task"]["sub_tasks"][1]["sub_task_id"]
            .as_str()
            .unwrap();

        assert_eq!(abandon_sub(master_id, sub_a)["_status"], 200);
        let again = abandon_sub(master_id, sub_a);
        assert_eq!(again["_status"], 409);

        assert_eq!(complete_sub(master_id, sub_b)["_status"], 200);
        let on_complete = abandon_sub(master_id, sub_b);
        assert_eq!(on_complete["_status"], 409);
    });
}

#[test]
fn abandon_sub_unknown_ids_return_404() {
    with_plan_task_sandbox(|_| {
        let v = abandon_sub("task_missing", "task_missing_sub_01");
        assert_eq!(v["_status"], 404);

        let created = create_master_with_subs("Exists", Some(&["A"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let missing = abandon_sub(master_id, "task_nosuch_sub_99");
        assert_eq!(missing["_status"], 404);
    });
}

#[test]
fn delete_master_removes_index_entry_and_task_directory() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete me", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(task_dir.is_dir());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert!(!task_dir.exists());
        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert!(index["tasks"].get(master_id).is_none());
        assert_eq!(get_by_id(master_id)["_status"], 404);
    });
}

#[test]
fn create_batch_failure_leaves_no_partial_commit() {
    with_plan_task_sandbox(|wb| {
        test_set_fail_batch_index(true);
        let v = create_master_with_subs("Fail batch", None);
        assert_eq!(v["_status"], 500);

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index["tasks"].as_object().map(|m| m.len()).unwrap_or(0), 0);

        let list = list_all();
        let list = list.as_array().expect("array");
        assert!(list.is_empty());
    });
}

#[test]
fn create_response_omits_completed_at_on_new_subs() {
    with_plan_task_sandbox(|_| {
        let v = create_master_with_subs("New", Some(&["Sub"]));
        let subs = v["task"]["sub_tasks"].as_array().unwrap();
        assert!(subs[0].get("completed_at").is_none() || subs[0]["completed_at"].is_null());
    });
}

fn seed_v2_plan_with_subs(
    wb: &Path,
    master_id: &str,
    title: &str,
    sub_tasks: &serde_json::Value,
    merge_index: bool,
) {
    let plan_tasks_dir = wb.join("plan_tasks");
    fs::create_dir_all(plan_tasks_dir.join("tasks").join(master_id)).expect("mkdir task");
    if merge_index {
        let index_path = plan_tasks_dir.join("index.json");
        let mut index: serde_json::Value = if index_path.is_file() {
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap_or_else(|_| {
                serde_json::json!({ "version": 2, "tasks": {} })
            })
        } else {
            fs::create_dir_all(plan_tasks_dir.join("tasks")).expect("mkdir tasks");
            serde_json::json!({ "version": 2, "tasks": {} })
        };
        index["tasks"][master_id] = serde_json::json!({
            "master_task_id": master_id,
            "title": title,
            "status": "incomplete",
            "created_at": "2026-07-08T00:00:00+00:00",
            "task_dir": format!("tasks/{master_id}")
        });
        fs::write(
            index_path,
            serde_json::to_string_pretty(&index).expect("serialize index"),
        )
        .expect("write index");
    }
    fs::write(
        plan_tasks_dir
            .join("tasks")
            .join(master_id)
            .join("sub_tasks.json"),
        serde_json::to_string_pretty(sub_tasks).expect("serialize subs"),
    )
    .expect("write sub_tasks");
    fs::write(
        plan_tasks_dir
            .join("tasks")
            .join(master_id)
            .join("plan.md"),
        "",
    )
    .expect("write plan.md");
}

fn subs_on_disk(wb: &Path, master_id: &str) -> serde_json::Value {
    let path = wb
        .join("plan_tasks")
        .join("tasks")
        .join(master_id)
        .join("sub_tasks.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).expect("parse sub_tasks.json")
}

#[test]
fn migrate_implicit_subs_removes_implicit_on_bootstrap() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_implicit",
            "Implicit plan",
            &serde_json::json!({
                "sub_tasks": [{
                    "sub_task_id": "task_implicit_sub_01",
                    "title": "Implicit plan",
                    "status": "incomplete",
                    "implicit": true,
                    "linked_archive_ids": []
                }]
            }),
            true,
        );

        let list = list_all();
        let listed = list.as_array().expect("array");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["migration_error"], false);
        assert_eq!(listed[0]["sub_tasks"].as_array().unwrap().len(), 0);

        let got = get_by_id("task_implicit");
        assert_eq!(got["migration_error"], false);
        assert!(got["sub_tasks"].as_array().unwrap().is_empty());

        let on_disk = subs_on_disk(wb, "task_implicit");
        assert_eq!(on_disk["sub_tasks"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn migrate_implicit_subs_preserves_explicit_subs() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_mixed",
            "Mixed",
            &serde_json::json!({
                "sub_tasks": [
                    {
                        "sub_task_id": "task_mixed_sub_01",
                        "title": "Implicit",
                        "status": "incomplete",
                        "implicit": true,
                        "linked_archive_ids": []
                    },
                    {
                        "sub_task_id": "task_mixed_sub_02",
                        "title": "Explicit",
                        "status": "incomplete",
                        "implicit": false,
                        "linked_archive_ids": []
                    }
                ]
            }),
            true,
        );

        let got = get_by_id("task_mixed");
        assert_eq!(got["migration_error"], false);
        let subs = got["sub_tasks"].as_array().unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["sub_task_id"], "task_mixed_sub_02");
        assert_eq!(subs[0]["implicit"], false);

        let on_disk = subs_on_disk(wb, "task_mixed");
        assert_eq!(on_disk["sub_tasks"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn migrate_implicit_subs_no_implicit_plans_unchanged() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_explicit",
            "Explicit only",
            &serde_json::json!({
                "sub_tasks": [{
                    "sub_task_id": "task_explicit_sub_01",
                    "title": "Keep",
                    "status": "incomplete",
                    "implicit": false,
                    "linked_archive_ids": []
                }]
            }),
            true,
        );

        let before = subs_on_disk(wb, "task_explicit");
        list_all();
        let after = subs_on_disk(wb, "task_explicit");
        assert_eq!(before, after);

        let got = get_by_id("task_explicit");
        assert_eq!(got["migration_error"], false);
        assert_eq!(got["sub_tasks"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn migrate_implicit_subs_idempotent_second_bootstrap() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_idempotent",
            "Idempotent",
            &serde_json::json!({
                "sub_tasks": [{
                    "sub_task_id": "task_idempotent_sub_01",
                    "title": "Idempotent",
                    "status": "incomplete",
                    "implicit": true,
                    "linked_archive_ids": []
                }]
            }),
            true,
        );

        list_all();
        let after_first = fs::read_to_string(
            wb.join("plan_tasks")
                .join("tasks")
                .join("task_idempotent")
                .join("sub_tasks.json"),
        )
        .unwrap();

        list_all();
        let after_second = fs::read_to_string(
            wb.join("plan_tasks")
                .join("tasks")
                .join("task_idempotent")
                .join("sub_tasks.json"),
        )
        .unwrap();

        assert_eq!(after_first, after_second);
        let got = get_by_id("task_idempotent");
        assert_eq!(got["migration_error"], false);
        assert!(got["sub_tasks"].as_array().unwrap().is_empty());
    });
}

#[test]
fn migrate_implicit_subs_marks_migration_error_on_corrupt_plan() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_corrupt_migrate",
            "Corrupt migrate",
            &serde_json::json!({ "sub_tasks": [] }),
            true,
        );
        fs::write(
            wb.join("plan_tasks")
                .join("tasks")
                .join("task_corrupt_migrate")
                .join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let list = list_all();
        let listed = list.as_array().expect("array");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["master_task_id"], "task_corrupt_migrate");
        assert_eq!(listed[0]["migration_error"], true);
    });
}

#[test]
fn migrate_implicit_subs_other_plans_unaffected_on_single_failure() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_ok",
            "OK plan",
            &serde_json::json!({
                "sub_tasks": [{
                    "sub_task_id": "task_ok_sub_01",
                    "title": "Explicit",
                    "status": "incomplete",
                    "implicit": false,
                    "linked_archive_ids": []
                }]
            }),
            true,
        );
        seed_v2_plan_with_subs(
            wb,
            "task_bad",
            "Bad plan",
            &serde_json::json!({ "sub_tasks": [] }),
            true,
        );
        fs::write(
            wb.join("plan_tasks")
                .join("tasks")
                .join("task_bad")
                .join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let list = list_all();
        let listed = list.as_array().expect("array");
        assert_eq!(listed.len(), 2);

        let ok = listed
            .iter()
            .find(|v| v["master_task_id"] == "task_ok")
            .expect("ok plan");
        assert_eq!(ok["migration_error"], false);
        assert_eq!(ok["sub_tasks"].as_array().unwrap().len(), 1);

        let bad = listed
            .iter()
            .find(|v| v["master_task_id"] == "task_bad")
            .expect("bad plan");
        assert_eq!(bad["migration_error"], true);
    });
}

#[test]
fn get_by_id_includes_migration_error_on_corrupt_plan() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_corrupt_get_migrate",
            "Corrupt get",
            &serde_json::json!({ "sub_tasks": [] }),
            true,
        );
        fs::write(
            wb.join("plan_tasks")
                .join("tasks")
                .join("task_corrupt_get_migrate")
                .join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let got = get_by_id("task_corrupt_get_migrate");
        assert_eq!(got["master_task_id"], "task_corrupt_get_migrate");
        assert_eq!(got["migration_error"], true);
        assert!(got.get("_status").is_none());
    });
}

#[test]
fn read_plan_md_missing_master_returns_404() {
    with_plan_task_sandbox(|_| {
        let v = read_plan_md("task_does_not_exist");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn update_plan_md_missing_master_returns_404() {
    with_plan_task_sandbox(|_| {
        let v = update_plan_md("task_does_not_exist", "# Plan");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn list_and_get_include_plan_md_and_migration_error() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Plan md fields", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let list = list_all();
        let listed = list.as_array().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["plan_md"], "");
        assert_eq!(listed[0]["migration_error"], false);

        let got = get_by_id(master_id);
        assert_eq!(got["plan_md"], "");
        assert_eq!(got["migration_error"], false);
    });
}

#[test]
fn update_plan_md_empty_round_trip() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Empty plan", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let updated = update_plan_md(master_id, "");
        assert_eq!(updated["_status"], 200);

        let read = read_plan_md(master_id);
        assert_eq!(read["_status"], 200);
        assert_eq!(read["plan_md"], "");

        let plan_path = wb
            .join("plan_tasks")
            .join("tasks")
            .join(master_id)
            .join("plan.md");
        assert_eq!(fs::read_to_string(plan_path).unwrap(), "");

        let got = get_by_id(master_id);
        assert_eq!(got["plan_md"], "");
    });
}

#[test]
fn update_plan_md_multiline_round_trip() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Markdown plan", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let content = "# Title\n\n## Section\n\n- item one\n- item two\n";

        let updated = update_plan_md(master_id, content);
        assert_eq!(updated["_status"], 200);

        let read = read_plan_md(master_id);
        assert_eq!(read["_status"], 200);
        assert_eq!(read["plan_md"], content);

        let plan_path = wb
            .join("plan_tasks")
            .join("tasks")
            .join(master_id)
            .join("plan.md");
        assert_eq!(fs::read_to_string(plan_path).unwrap(), content);

        let got = get_by_id(master_id);
        assert_eq!(got["plan_md"], content);

        let list = list_all();
        let listed = list.as_array().unwrap();
        assert_eq!(listed[0]["plan_md"], content);
    });
}

#[test]
fn update_plan_md_io_failure_returns_500_without_corrupting_index() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("IO fail", Some(&["Sub"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let index_before = fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap();
        let subs_before = subs_on_disk(wb, master_id);

        test_set_fail_batch_plan_md(true);
        let v = update_plan_md(master_id, "should not persist");
        assert_eq!(v["_status"], 500);
        assert!(v.get("error").is_some());

        let index_after = fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap();
        let subs_after = subs_on_disk(wb, master_id);
        assert_eq!(index_before, index_after);
        assert_eq!(subs_before, subs_after);
    });
}

#[test]
fn title_unit_count_chinese_and_english() {
    assert_eq!(title_unit_count("预习第三章内容大纲"), 9);
    assert_eq!(title_unit_count("预习：第三章"), 5);
    assert_eq!(title_unit_count("Short title"), 2);
    assert_eq!(
        title_unit_count("one two three four five six seven eight nine ten eleven"),
        11
    );
    assert_eq!(title_unit_count("计划 MCP rollout"), 4);
}

#[test]
fn create_rejects_title_over_twenty_units() {
    with_plan_task_sandbox(|_| {
        let long_en = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone";
        let v = create_master_with_subs(long_en, None);
        assert_eq!(v["_status"], 400);
        assert!(
            v["error"]
                .as_str()
                .unwrap_or("")
                .contains("Title too long"),
            "error={:?}",
            v["error"]
        );

        let long_zh = "一二三四五六七八九十十一十二十三十四十五十六十七十八十九二十廿一";
        let v2 = create_master_with_subs(long_zh, None);
        assert_eq!(v2["_status"], 400);
    });
}

#[test]
fn create_with_plan_md_persists_to_disk_and_list() {
    with_plan_task_sandbox(|wb| {
        let content = "## Notes\n\nHello plan";
        let created = create_master_with_subs_and_plan("With plan", None, content);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().unwrap();

        let plan_path = wb
            .join("plan_tasks")
            .join("tasks")
            .join(master_id)
            .join("plan.md");
        assert_eq!(fs::read_to_string(&plan_path).unwrap(), content);

        let got = get_by_id(master_id);
        assert_eq!(got["plan_md"], content);
        let list = list_all();
        assert_eq!(list.as_array().unwrap()[0]["plan_md"], content);
    });
}

#[test]
fn migrate_implicit_subs_write_failure_marks_migration_error() {
    with_plan_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_write_fail",
            "Write fail",
            &serde_json::json!({
                "sub_tasks": [{
                    "sub_task_id": "task_write_fail_sub_01",
                    "title": "Implicit",
                    "status": "incomplete",
                    "implicit": true,
                    "linked_archive_ids": []
                }]
            }),
            true,
        );
        test_set_fail_migrate_implicit_write(true);

        let list = list_all();
        let listed = list.as_array().expect("array");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["migration_error"], true);
    });
}

#[test]
fn update_master_title_success_trims_and_persists() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Old title", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().unwrap();

        let updated = update_master_title(master_id, "  New title  ");
        assert_eq!(updated["_status"], 200);
        assert_eq!(master_from_value(&updated)["title"], "New title");

        let got = get_by_id(master_id);
        assert_eq!(got["title"], "New title");
        let listed = list_all();
        assert_eq!(listed.as_array().unwrap()[0]["title"], "New title");
    });
}

#[test]
fn update_master_title_blank_after_trim_returns_400() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Keep me", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = update_master_title(master_id, "   \t  ");
        assert_eq!(v["_status"], 400);
        assert_eq!(v["error"], "Missing title");
        assert_eq!(get_by_id(master_id)["title"], "Keep me");
    });
}

#[test]
fn update_master_title_rejects_over_twenty_units() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Short", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let long_en = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty twentyone";

        let v = update_master_title(master_id, long_en);
        assert_eq!(v["_status"], 400);
        assert!(
            v["error"]
                .as_str()
                .unwrap_or("")
                .contains("Title too long"),
            "error={:?}",
            v["error"]
        );
        assert_eq!(get_by_id(master_id)["title"], "Short");
    });
}

#[test]
fn update_master_title_unknown_master_returns_404() {
    with_plan_task_sandbox(|_| {
        let v = update_master_title("task_nonexistent_aaaaaaaaaaaaaaaa", "New");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

fn attachments_dir(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("plan_tasks")
        .join("tasks")
        .join(master_id)
        .join("attachments")
}

fn attachments_json_path(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("plan_tasks")
        .join("tasks")
        .join(master_id)
        .join("attachments.json")
}

fn load_attachments_file(wb: &Path, master_id: &str) -> AttachmentsFile {
    let text = fs::read_to_string(attachments_json_path(wb, master_id)).expect("attachments.json");
    serde_json::from_str(&text).expect("parse attachments.json")
}

#[test]
fn add_attachment_md_copies_and_writes_manifest() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Attach me", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let content = "# Notes\n\nhello";

        let v = add_attachment(master_id, "notes.md", content);
        assert_eq!(v["_status"], 201);
        assert_eq!(v["file_name"], "notes.md");
        assert_eq!(v["original_file_name"], "notes.md");
        assert!(is_iso8601(v["added_at"].as_str().unwrap()));

        let on_disk = attachments_dir(wb, master_id).join("notes.md");
        assert!(on_disk.is_file());
        assert_eq!(fs::read_to_string(&on_disk).unwrap(), content);

        let manifest = load_attachments_file(wb, master_id);
        assert_eq!(manifest.attachments.len(), 1);
        assert_eq!(manifest.attachments[0].file_name, "notes.md");
        assert_eq!(manifest.attachments[0].original_file_name, "notes.md");
        assert!(is_iso8601(&manifest.attachments[0].added_at));
    });
}

#[test]
fn add_attachment_stem_conflict_appends_numeric_suffix() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Conflict", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let first = add_attachment(master_id, "notes.md", "one");
        assert_eq!(first["_status"], 201);
        assert_eq!(first["file_name"], "notes.md");

        let second = add_attachment(master_id, "notes.md", "two");
        assert_eq!(second["_status"], 201);
        assert_eq!(second["file_name"], "notes-1.md");
        assert_eq!(second["original_file_name"], "notes.md");

        let third = add_attachment(master_id, "notes.md", "three");
        assert_eq!(third["_status"], 201);
        assert_eq!(third["file_name"], "notes-2.md");

        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("notes.md")).unwrap(),
            "one"
        );
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("notes-1.md")).unwrap(),
            "two"
        );
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("notes-2.md")).unwrap(),
            "three"
        );

        let manifest = load_attachments_file(wb, master_id);
        let names: Vec<&str> = manifest
            .attachments
            .iter()
            .map(|e| e.file_name.as_str())
            .collect();
        assert_eq!(names, vec!["notes.md", "notes-1.md", "notes-2.md"]);
    });
}

#[test]
fn add_attachment_allows_empty_md_content() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Empty md", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = add_attachment(master_id, "empty.md", "");
        assert_eq!(v["_status"], 201);
        assert_eq!(v["file_name"], "empty.md");
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("empty.md")).unwrap(),
            ""
        );
        let manifest = load_attachments_file(wb, master_id);
        assert_eq!(manifest.attachments.len(), 1);
    });
}

#[test]
fn add_attachment_accepts_uppercase_md_extension() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Upper ext", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = add_attachment(master_id, "NOTES.MD", "upper");
        assert_eq!(v["_status"], 201);
        let stored = v["file_name"].as_str().unwrap();
        assert!(
            Path::new(stored)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("md")),
            "stored={stored}"
        );
        assert!(attachments_dir(wb, master_id).join(stored).is_file());
        let manifest = load_attachments_file(wb, master_id);
        assert_eq!(manifest.attachments.len(), 1);
        assert_eq!(manifest.attachments[0].file_name, stored);
        assert_eq!(manifest.attachments[0].original_file_name, "NOTES.MD");
    });
}

#[test]
fn add_attachment_rejects_non_md_without_side_effects() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Reject txt", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = add_attachment(master_id, "notes.txt", "nope");
        assert_eq!(v["_status"], 400);
        assert!(v.get("error").is_some());

        assert!(!attachments_dir(wb, master_id).exists());
        assert!(!attachments_json_path(wb, master_id).exists());
    });
}

#[test]
fn add_attachment_rejects_path_separators_and_empty_name() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Bad names", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        for bad in ["../escape.md", "a/b.md", "a\\b.md", "", "   ", ".md"] {
            let v = add_attachment(master_id, bad, "x");
            assert_eq!(v["_status"], 400, "bad={bad:?}");
            assert!(v.get("error").is_some());
        }

        assert!(!attachments_dir(wb, master_id).exists());
        assert!(!attachments_json_path(wb, master_id).exists());
    });
}

#[test]
fn add_attachment_manifest_write_failure_rolls_back_copy() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Rollback", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let ok = add_attachment(master_id, "keep.md", "keep-me");
        assert_eq!(ok["_status"], 201);
        let before = load_attachments_file(wb, master_id);

        test_set_fail_add_attachment_manifest(true);
        let v = add_attachment(master_id, "new.md", "should-roll-back");
        assert_eq!(v["_status"], 500);
        assert!(v.get("error").is_some());

        assert!(!attachments_dir(wb, master_id).join("new.md").exists());
        let after = load_attachments_file(wb, master_id);
        assert_eq!(before, after);
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("keep.md")).unwrap(),
            "keep-me"
        );
    });
}

#[test]
fn add_attachment_unknown_master_returns_404_without_writes() {
    with_plan_task_sandbox(|wb| {
        let v = add_attachment("task_nonexistent_aaaaaaaaaaaaaaaa", "notes.md", "x");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
        assert!(!wb
            .join("plan_tasks")
            .join("tasks")
            .join("task_nonexistent_aaaaaaaaaaaaaaaa")
            .exists());
    });
}

#[test]
fn add_attachment_unwritable_task_dir_fails_without_half_success() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Readonly dir", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);

        let mut perms = fs::metadata(&task_dir).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&task_dir, perms).unwrap();

        let v = add_attachment(master_id, "notes.md", "x");
        assert_eq!(v["_status"], 500);
        assert!(v.get("error").is_some());
        assert!(!attachments_dir(wb, master_id).join("notes.md").exists());
        assert!(!attachments_json_path(wb, master_id).exists());

        let mut perms = fs::metadata(&task_dir).unwrap().permissions();
        perms.set_readonly(false);
        fs::set_permissions(&task_dir, perms).unwrap();
    });
}

#[test]
fn list_attachments_returns_manifest_entries_not_directory_scan() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("List attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let a = add_attachment(master_id, "a.md", "one");
        let b = add_attachment(master_id, "b.md", "two");
        assert_eq!(a["_status"], 201);
        assert_eq!(b["_status"], 201);

        // Orphan on disk must not appear — list is attachments.json SSOT.
        fs::create_dir_all(attachments_dir(wb, master_id)).unwrap();
        fs::write(attachments_dir(wb, master_id).join("orphan.md"), "ghost").unwrap();

        let v = list_attachments(master_id);
        assert_eq!(v["_status"], 200);
        let items = v["attachments"].as_array().expect("attachments array");
        assert_eq!(items.len(), 2);
        let names: Vec<&str> = items
            .iter()
            .map(|e| e["file_name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["a.md", "b.md"]);
        assert!(items.iter().all(|e| e.get("original_file_name").is_some()));
        assert!(items
            .iter()
            .all(|e| is_iso8601(e["added_at"].as_str().unwrap())));
    });
}

#[test]
fn list_attachments_missing_manifest_or_empty_returns_empty_collection() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Empty list", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        assert!(!attachments_json_path(wb, master_id).exists());
        let missing = list_attachments(master_id);
        assert_eq!(missing["_status"], 200);
        assert_eq!(
            missing["attachments"].as_array().expect("array").len(),
            0
        );

        // Explicit empty manifest is also an empty collection (distinguishable success).
        fs::write(
            attachments_json_path(wb, master_id),
            r#"{"attachments":[]}"#,
        )
        .unwrap();
        let empty = list_attachments(master_id);
        assert_eq!(empty["_status"], 200);
        assert_eq!(empty["attachments"].as_array().expect("array").len(), 0);
    });
}

#[test]
fn list_attachments_unknown_master_returns_404() {
    with_plan_task_sandbox(|_| {
        let v = list_attachments("task_nonexistent_aaaaaaaaaaaaaaaa");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

#[test]
fn read_attachment_returns_manifest_file_content() {
    with_plan_task_sandbox(|_| {
        let created = create_master_with_subs("Read attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let content = "# Body\n\nline";

        let added = add_attachment(master_id, "notes.md", content);
        assert_eq!(added["_status"], 201);

        let v = read_attachment(master_id, "notes.md");
        assert_eq!(v["_status"], 200);
        assert_eq!(v["file_name"], "notes.md");
        assert_eq!(v["content"], content);
    });
}

#[test]
fn read_attachment_rejects_non_manifest_target() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Read reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        add_attachment(master_id, "listed.md", "ok");

        fs::create_dir_all(attachments_dir(wb, master_id)).unwrap();
        fs::write(attachments_dir(wb, master_id).join("orphan.md"), "ghost").unwrap();

        let v = read_attachment(master_id, "orphan.md");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn save_attachment_writes_content_without_touching_plan_md() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs_and_plan("Save attach", None, "# Plan body");
        let master_id = created["master_task_id"].as_str().unwrap();
        let plan_path = wb
            .join("plan_tasks")
            .join("tasks")
            .join(master_id)
            .join("plan.md");
        let plan_before = fs::read_to_string(&plan_path).unwrap();

        let added = add_attachment(master_id, "notes.md", "old");
        assert_eq!(added["_status"], 201);

        let v = save_attachment(master_id, "notes.md", "new content");
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("notes.md")).unwrap(),
            "new content"
        );
        assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan_before);
        assert_eq!(read_plan_md(master_id)["plan_md"], "# Plan body");
    });
}

#[test]
fn save_attachment_allows_empty_content() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Save empty", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        add_attachment(master_id, "notes.md", "had text");

        let v = save_attachment(master_id, "notes.md", "");
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("notes.md")).unwrap(),
            ""
        );
    });
}

#[test]
fn save_attachment_rejects_non_manifest_target() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Save reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        add_attachment(master_id, "listed.md", "ok");

        fs::create_dir_all(attachments_dir(wb, master_id)).unwrap();
        fs::write(attachments_dir(wb, master_id).join("orphan.md"), "ghost").unwrap();

        let v = save_attachment(master_id, "orphan.md", "overwrite");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("orphan.md")).unwrap(),
            "ghost"
        );
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("listed.md")).unwrap(),
            "ok"
        );
    });
}

#[test]
fn delete_attachment_removes_manifest_entry_and_file() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let added = add_attachment(master_id, "notes.md", "to-delete");
        assert_eq!(added["_status"], 201);
        assert!(attachments_dir(wb, master_id).join("notes.md").is_file());

        let v = delete_attachment(master_id, "notes.md");
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);

        assert!(!attachments_dir(wb, master_id).join("notes.md").exists());
        let listed = list_attachments(master_id);
        assert_eq!(listed["_status"], 200);
        let names: Vec<&str> = listed["attachments"]
            .as_array()
            .expect("attachments array")
            .iter()
            .map(|e| e["file_name"].as_str().unwrap())
            .collect();
        assert!(!names.contains(&"notes.md"));
        assert!(load_attachments_file(wb, master_id)
            .attachments
            .iter()
            .all(|e| e.file_name != "notes.md"));
    });
}

#[test]
fn delete_attachment_preserves_other_entries() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete keeps others", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        assert_eq!(add_attachment(master_id, "keep-a.md", "a")["_status"], 201);
        assert_eq!(add_attachment(master_id, "drop.md", "drop-me")["_status"], 201);
        assert_eq!(add_attachment(master_id, "keep-b.md", "b")["_status"], 201);
        let before = load_attachments_file(wb, master_id);

        let v = delete_attachment(master_id, "drop.md");
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);

        assert!(!attachments_dir(wb, master_id).join("drop.md").exists());
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("keep-a.md")).unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("keep-b.md")).unwrap(),
            "b"
        );

        let after = load_attachments_file(wb, master_id);
        let names: Vec<&str> = after
            .attachments
            .iter()
            .map(|e| e.file_name.as_str())
            .collect();
        assert_eq!(names, vec!["keep-a.md", "keep-b.md"]);
        assert_eq!(after.attachments[0], before.attachments[0]);
        assert_eq!(after.attachments[1], before.attachments[2]);
    });
}

#[test]
fn delete_attachment_rejects_non_manifest_target_without_side_effects() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert_eq!(add_attachment(master_id, "listed.md", "ok")["_status"], 201);

        fs::create_dir_all(attachments_dir(wb, master_id)).unwrap();
        fs::write(attachments_dir(wb, master_id).join("orphan.md"), "ghost").unwrap();
        let before = load_attachments_file(wb, master_id);

        let v = delete_attachment(master_id, "orphan.md");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());

        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("orphan.md")).unwrap(),
            "ghost"
        );
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("listed.md")).unwrap(),
            "ok"
        );
        assert_eq!(load_attachments_file(wb, master_id), before);
    });
}

#[test]
fn delete_attachment_file_delete_failure_restores_manifest() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete rollback", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        assert_eq!(add_attachment(master_id, "keep.md", "keep-me")["_status"], 201);
        assert_eq!(add_attachment(master_id, "drop.md", "drop-me")["_status"], 201);
        let before = load_attachments_file(wb, master_id);

        test_set_fail_delete_attachment_file(true);
        let v = delete_attachment(master_id, "drop.md");
        assert_eq!(v["_status"], 500);
        assert!(v.get("error").is_some());

        let after = load_attachments_file(wb, master_id);
        assert_eq!(after, before);
        assert!(attachments_dir(wb, master_id).join("drop.md").is_file());
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("drop.md")).unwrap(),
            "drop-me"
        );
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("keep.md")).unwrap(),
            "keep-me"
        );
        // No half-success: neither "manifest without file" nor "file without manifest".
        assert!(after.attachments.iter().any(|e| e.file_name == "drop.md"));
        assert!(attachments_dir(wb, master_id).join("drop.md").exists());
    });
}

#[test]
fn delete_attachment_unknown_master_returns_404() {
    with_plan_task_sandbox(|_| {
        let v = delete_attachment("task_nonexistent_aaaaaaaaaaaaaaaa", "notes.md");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

#[test]
fn delete_master_cascades_attachments_dir_and_manifest() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete with attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let add = add_attachment(master_id, "notes.md", "# keep\n");
        assert_eq!(add["_status"], 201);

        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        let att_dir = attachments_dir(wb, master_id);
        let att_json = attachments_json_path(wb, master_id);
        assert!(task_dir.is_dir());
        assert!(att_dir.is_dir());
        assert!(att_json.is_file());
        assert!(att_dir.join("notes.md").is_file());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert!(!task_dir.exists());
        assert!(!att_dir.exists());
        assert!(!att_json.exists());
        assert_eq!(get_by_id(master_id)["_status"], 404);

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("plan_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert!(index["tasks"].get(master_id).is_none());
    });
}

#[test]
fn delete_master_without_attachments_matches_existing_behavior() {
    with_plan_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete bare", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("plan_tasks").join("tasks").join(master_id);
        assert!(task_dir.is_dir());
        assert!(!attachments_dir(wb, master_id).exists());
        assert!(!attachments_json_path(wb, master_id).exists());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert!(!task_dir.exists());
        assert!(!attachments_dir(wb, master_id).exists());
        assert!(!attachments_json_path(wb, master_id).exists());
        assert_eq!(get_by_id(master_id)["_status"], 404);
    });
}
