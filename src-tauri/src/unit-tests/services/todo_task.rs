use super::*;
use crate::services::todo_task::types::{
    AttachmentsFile, CommentEntry, CommentsFile, IndexEntry, MasterTaskStatus, SubTask,
    SubTaskStatus, SubTasksFile, DEFAULT_CATEGORY_ID, DEFAULT_CATEGORY_NAME,
};
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::config::paths;
use crate::config::settings::{self, default_cache_dir};
use crate::services::todo_task::test_reset_all_injection_flags;
use crate::test_support::TestSandbox;

fn with_todo_task_sandbox<F: FnOnce(&Path)>(f: F) {
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
        .join("todo_tasks")
        .join("todo_tasks.json")
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
}

fn prod_cache_plan_tasks_mtime() -> Option<SystemTime> {
    default_cache_dir()
        .join("todo_tasks.json")
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
}

#[test]
fn plan_tasks_path_is_under_workbench_knowledge_root() {
    with_todo_task_sandbox(|wb| {
        let path = paths::plan_tasks_path().expect("path");
        assert_eq!(path, wb.join("todo_tasks").join("todo_tasks.json"));
        let cache = paths::cache_dir().expect("cache");
        assert_ne!(path, cache.join("todo_tasks.json"));
        assert!(!path.starts_with(&cache));
    });
}

#[test]
fn first_write_creates_plan_tasks_directory() {
    with_todo_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("todo_tasks");
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
    with_todo_task_sandbox(|_| {
        let v = list_all();
        assert_eq!(v.as_array().expect("array").len(), 0);
        assert!(v.get("_status").is_none());
    });
}

#[test]
fn create_single_explicit_sub_implicit_false() {
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|wb| {
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

        let path = wb.join("todo_tasks").join("index.json");
        assert!(path.is_file());
        let cache = paths::cache_dir().expect("cache");
        assert!(!path.starts_with(&cache));
    });
}

#[test]
fn plan_task_tests_do_not_touch_prod_plan_tasks_or_cache() {
    let before_wb = prod_plan_tasks_mtime();
    let before_cache = prod_cache_plan_tasks_mtime();
    with_todo_task_sandbox(|_| {
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
    let prod_plan_tasks = prod_wb.join("todo_tasks").join("todo_tasks.json");
    assert!(sandbox.assert_not_prod_path(&prod_plan_tasks).is_err());
    let prod_cache_plan = sandbox.prod_cache_dir().join("todo_tasks.json");
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
            .join("todo_tasks")
            .join("todo_tasks.json")
    );
    assert_ne!(plan_path, sandbox.prod_cache_dir().join("todo_tasks.json"));
}

#[test]
fn create_without_sub_titles_yields_empty_sub_tasks() {
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
        let empty: &[&str] = &[];
        let v = create_master_with_subs("Empty slice", Some(empty));
        assert_eq!(v["_status"], 201);
        let subs = master_from_value(&v)["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
        assert_eq!(master_from_value(&v)["status"], "incomplete");
    });
}

#[test]
fn stored_complete_with_empty_sub_tasks_is_preserved_without_recompute() {
    with_todo_task_sandbox(|_| {
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
        assert_eq!(got["status"], "complete");
        assert_eq!(got["sub_tasks"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn completing_all_subs_does_not_recompute_master_status() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("All done", Some(&["A", "B"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        for sub in created["task"]["sub_tasks"].as_array().unwrap() {
            let sub_id = sub["sub_task_id"].as_str().unwrap();
            let v = complete_sub(master_id, sub_id);
            assert_eq!(v["_status"], 200);
        }
        let got = get_by_id(master_id);
        assert_eq!(got["status"], "incomplete");
    });
}

#[test]
fn explicit_abandoned_write_keeps_index_and_authority_aligned() {
    with_todo_task_sandbox(|wb| {
        let master_id = "task_abandoned_explicit";
        let mut entry = sample_index_entry(master_id);
        entry.status = MasterTaskStatus::Abandoned;
        test_run_write_task_batch(
            master_id,
            &entry,
            &SubTasksFile { sub_tasks: vec![] },
            "",
        )
        .expect("batch write");

        let got = get_by_id(master_id);
        assert_eq!(got["status"], "abandoned");

        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(index["tasks"][master_id]["status"], "abandoned");
        assert_eq!(index["tasks"][master_id]["status"], got["status"]);
    });
}

#[test]
fn set_master_status_allows_tri_state_mutual_transitions() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Set status", Some(&["A"]));
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("master_task_id");

        for target in ["complete", "abandoned", "incomplete"] {
            let v = set_master_status(master_id, target);
            assert_eq!(v["_status"], 200, "target={target} body={v}");
            assert_eq!(master_from_value(&v)["status"], target);

            let got = get_by_id(master_id);
            assert_eq!(got["status"], target);

            let index: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(index["tasks"][master_id]["status"], target);
            assert_eq!(index["tasks"][master_id]["status"], got["status"]);
        }
    });
}

#[test]
fn set_master_status_rejects_unknown_status() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Bad status", Some(&["A"]));
        let master_id = created["master_task_id"].as_str().expect("master_task_id");
        let v = set_master_status(master_id, "done");
        assert_eq!(v["_status"], 400);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn create_multiple_subs_increments_sub_suffix() {
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Sub lookup", Some(&["Sub"]));
        let sub_id = created["sub_task_id"].as_str().unwrap();
        let got = get_by_id(sub_id);
        assert_eq!(got["master_task_id"], created["master_task_id"]);
        assert_eq!(got["sub_tasks"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn list_all_returns_master_trees() {
    with_todo_task_sandbox(|_| {
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
fn complete_sub_does_not_mark_master_complete_when_all_subs_done() {
    with_todo_task_sandbox(|_| {
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
        assert_eq!(got["status"], "incomplete");
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
    with_todo_task_sandbox(|_| {
        let v = complete_sub("task_missing", "task_missing_sub_01");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn get_by_id_unknown_returns_404() {
    with_todo_task_sandbox(|_| {
        let v = get_by_id("task_does_not_exist");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn link_archive_updates_reverse_index() {
    with_todo_task_sandbox(|_| {
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
        let plan_tasks_dir = wb.join("todo_tasks");
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
    with_todo_task_sandbox(|wb| {
        let master_id = "task_corrupt_get";
        let plan_tasks_dir = wb.join("todo_tasks");
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
    let index_path = wb.join("todo_tasks").join("index.json");
    let text = fs::read_to_string(&index_path).expect("read index.json");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse index.json");
    parsed["version"].as_u64().expect("index version") as u32
}

#[test]
fn bootstrap_deletes_wb_and_cache_v1_on_storage_read_entry() {
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|wb| {
        let wb_v1 = paths::plan_tasks_path().expect("wb v1 path");
        let cache_v1 = paths::cache_plan_tasks_v1_path().expect("cache v1 path");
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
fn bootstrap_creates_plan_tasks_and_tasks_dirs() {
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
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("todo_tasks");
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
            plan_tasks_dir.join("tasks").join("task_keepme").join("todo.md"),
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
    with_todo_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("todo_tasks");
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
    with_todo_task_sandbox(|wb| {
        let plan_tasks_dir = wb.join("todo_tasks");
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
        category_id: DEFAULT_CATEGORY_ID.to_string(),
    }
}

fn sample_sub_tasks(master_id: &str) -> SubTasksFile {
    SubTasksFile {
        sub_tasks: vec![SubTask {
            sub_task_id: format!("{master_id}_sub_01"),
            title: Some("Batch task".to_string()),
            content: None,
            status: SubTaskStatus::Incomplete,
            implicit: true,
            linked_archive_ids: vec![],
            completed_at: None,
        }],
    }
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
fn add_sub_appends_incomplete_sub_without_rewriting_master_status() {
    with_todo_task_sandbox(|_| {
        let master_id = "task_add_keeps_complete";
        let mut entry = sample_index_entry(master_id);
        entry.status = MasterTaskStatus::Complete;
        let mut subs = sample_sub_tasks(master_id);
        subs.sub_tasks[0].implicit = false;
        subs.sub_tasks[0].status = SubTaskStatus::Complete;
        subs.sub_tasks[0].completed_at = Some("2026-07-08T01:00:00+00:00".to_string());
        test_run_write_task_batch(master_id, &entry, &subs, "").expect("batch write");

        let got_before = get_by_id(master_id);
        assert_eq!(got_before["status"], "complete");

        let added = add_sub(master_id, "B", None);
        assert_eq!(added["_status"], 201);
        assert!(added.get("sub_task_id").is_some());
        let task = master_from_value(&added);
        assert_eq!(task["status"], "complete");
        let subs = task["sub_tasks"].as_array().unwrap();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[1]["status"], "incomplete");
        assert_eq!(subs[1]["implicit"], false);
    });
}

#[test]
fn add_sub_with_content_persists_and_load_roundtrips() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Content master", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().unwrap();

        let added = add_sub(master_id, "With body", Some("persist me"));
        assert_eq!(added["_status"], 201);
        let task = master_from_value(&added);
        let subs = task["sub_tasks"].as_array().unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["title"], "With body");
        assert_eq!(subs[0]["content"], "persist me");

        let got = get_by_id(master_id);
        assert_eq!(got["sub_tasks"][0]["content"], "persist me");

        let on_disk = subs_on_disk(wb, master_id);
        assert_eq!(on_disk["sub_tasks"][0]["content"], "persist me");
    });
}

#[test]
fn add_sub_without_content_matches_title_only_behavior() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Title only master", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let added = add_sub(master_id, "Just title", None);
        assert_eq!(added["_status"], 201);
        let task = master_from_value(&added);
        let sub = &task["sub_tasks"][0];
        assert_eq!(sub["title"], "Just title");
        assert!(sub.get("content").is_none() || sub["content"].is_null());

        let on_disk = subs_on_disk(wb, master_id);
        assert!(
            on_disk["sub_tasks"][0].get("content").is_none(),
            "absent content must not be forced onto disk"
        );
    });
}

#[test]
fn load_legacy_sub_tasks_missing_content_reads_as_absent() {
    with_todo_task_sandbox(|wb| {
        let master_id = "task_legacy_content";
        seed_v2_plan_with_subs(
            wb,
            master_id,
            "Legacy",
            &serde_json::json!({
                "sub_tasks": [{
                    "sub_task_id": "task_legacy_content_sub_01",
                    "title": "Old sub",
                    "status": "incomplete",
                    "implicit": false,
                    "linked_archive_ids": []
                }]
            }),
            true,
        );

        let got = get_by_id(master_id);
        assert!(got.get("_status").is_none() || got["_status"] == 200);
        assert_eq!(got["master_task_id"], master_id);
        let sub = &got["sub_tasks"][0];
        assert_eq!(sub["title"], "Old sub");
        assert!(sub.get("content").is_none() || sub["content"].is_null());
    });
}

#[test]
fn update_sub_title_sets_optional_content() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Update content", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let added = add_sub(master_id, "Sub", Some("v1"));
        let sub_id = added["sub_task_id"].as_str().unwrap();

        let updated = update_sub_title(master_id, sub_id, "Sub", Some("v2"));
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["task"]["sub_tasks"][0]["content"], "v2");
    });
}

#[test]
fn update_sub_title_empty_content_clears() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Clear content", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let added = add_sub(master_id, "Sub", Some("wipe me"));
        let sub_id = added["sub_task_id"].as_str().unwrap();

        let cleared = update_sub_title(master_id, sub_id, "Sub", Some(""));
        assert_eq!(cleared["_status"], 200);
        let sub = &cleared["task"]["sub_tasks"][0];
        assert!(sub.get("content").is_none() || sub["content"].is_null() || sub["content"] == "");

        let on_disk = subs_on_disk(wb, master_id);
        assert!(
            on_disk["sub_tasks"][0].get("content").is_none()
                || on_disk["sub_tasks"][0]["content"] == "",
            "cleared content must not remain as non-empty on disk"
        );
    });
}

#[test]
fn update_sub_title_omitted_content_leaves_existing() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Omit content", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let added = add_sub(master_id, "Old", Some("keep"));
        let sub_id = added["sub_task_id"].as_str().unwrap();

        let renamed = update_sub_title(master_id, sub_id, "New", None);
        assert_eq!(renamed["_status"], 200);
        assert_eq!(renamed["task"]["sub_tasks"][0]["title"], "New");
        assert_eq!(renamed["task"]["sub_tasks"][0]["content"], "keep");
    });
}

#[test]
fn update_sub_title_blank_title_returns_400_even_with_content() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Blank title", Some(&["A"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let sub_id = created["task"]["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .unwrap();

        let v = update_sub_title(master_id, sub_id, "  ", Some("nope"));
        assert_eq!(v["_status"], 400);
        assert_eq!(v["error"], "Missing title");
    });
}

#[test]
fn update_sub_title_unknown_sub_returns_404() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Unknown sub", Some(&["A"]));
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = update_sub_title(master_id, "task_missing_sub_01", "New", None);
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

#[test]
fn complete_sub_partial_multi_sub_keeps_master_incomplete() {
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
fn delete_sub_allows_delete_to_empty_without_rewriting_master_status() {
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
fn abandon_sub_marks_sub_abandoned_without_rewriting_master_status() {
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete me", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        assert!(task_dir.is_dir());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert!(!task_dir.exists());
        let index: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert!(index["tasks"].get(master_id).is_none());
        assert_eq!(get_by_id(master_id)["_status"], 404);
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

#[test]
fn create_response_omits_completed_at_on_new_subs() {
    with_todo_task_sandbox(|_| {
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
    let plan_tasks_dir = wb.join("todo_tasks");
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
            .join("todo.md"),
        "",
    )
    .expect("write plan.md");
}

fn subs_on_disk(wb: &Path, master_id: &str) -> serde_json::Value {
    let path = wb
        .join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("sub_tasks.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).expect("parse sub_tasks.json")
}

#[test]
fn migrate_implicit_subs_removes_implicit_on_bootstrap() {
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|wb| {
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
            wb.join("todo_tasks")
                .join("tasks")
                .join("task_idempotent")
                .join("sub_tasks.json"),
        )
        .unwrap();

        list_all();
        let after_second = fs::read_to_string(
            wb.join("todo_tasks")
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
    with_todo_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_corrupt_migrate",
            "Corrupt migrate",
            &serde_json::json!({ "sub_tasks": [] }),
            true,
        );
        fs::write(
            wb.join("todo_tasks")
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
    with_todo_task_sandbox(|wb| {
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
            wb.join("todo_tasks")
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
    with_todo_task_sandbox(|wb| {
        seed_v2_plan_with_subs(
            wb,
            "task_corrupt_get_migrate",
            "Corrupt get",
            &serde_json::json!({ "sub_tasks": [] }),
            true,
        );
        fs::write(
            wb.join("todo_tasks")
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
fn read_todo_md_missing_master_returns_404() {
    with_todo_task_sandbox(|_| {
        let v = read_todo_md("task_does_not_exist");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn update_todo_md_missing_master_returns_404() {
    with_todo_task_sandbox(|_| {
        let v = update_todo_md("task_does_not_exist", "# Plan");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn list_and_get_include_todo_md_and_migration_error() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Plan md fields", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let list = list_all();
        let listed = list.as_array().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0]["todo_md"], "");
        assert_eq!(listed[0]["migration_error"], false);

        let got = get_by_id(master_id);
        assert_eq!(got["todo_md"], "");
        assert_eq!(got["migration_error"], false);
    });
}

#[test]
fn update_todo_md_empty_round_trip() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Empty plan", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let updated = update_todo_md(master_id, "");
        assert_eq!(updated["_status"], 200);

        let read = read_todo_md(master_id);
        assert_eq!(read["_status"], 200);
        assert_eq!(read["todo_md"], "");

        let plan_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(master_id)
            .join("todo.md");
        assert_eq!(fs::read_to_string(plan_path).unwrap(), "");

        let got = get_by_id(master_id);
        assert_eq!(got["todo_md"], "");
    });
}

#[test]
fn update_todo_md_multiline_round_trip() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Markdown plan", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let content = "# Title\n\n## Section\n\n- item one\n- item two\n";

        let updated = update_todo_md(master_id, content);
        assert_eq!(updated["_status"], 200);

        let read = read_todo_md(master_id);
        assert_eq!(read["_status"], 200);
        assert_eq!(read["todo_md"], content);

        let plan_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(master_id)
            .join("todo.md");
        assert_eq!(fs::read_to_string(plan_path).unwrap(), content);

        let got = get_by_id(master_id);
        assert_eq!(got["todo_md"], content);

        let list = list_all();
        let listed = list.as_array().unwrap();
        assert_eq!(listed[0]["todo_md"], content);
    });
}

#[test]
fn update_todo_md_io_failure_returns_500_without_corrupting_index() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("IO fail", Some(&["Sub"]));
        let master_id = created["master_task_id"].as_str().unwrap();
        let index_before = fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap();
        let subs_before = subs_on_disk(wb, master_id);

        test_set_fail_batch_todo_md(true);
        let v = update_todo_md(master_id, "should not persist");
        assert_eq!(v["_status"], 500);
        assert!(v.get("error").is_some());

        let index_after = fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap();
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
    with_todo_task_sandbox(|_| {
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
fn create_with_todo_md_persists_to_disk_and_list() {
    with_todo_task_sandbox(|wb| {
        let content = "## Notes\n\nHello plan";
        let created = create_master_with_subs_and_todo("With plan", None, content);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().unwrap();

        let plan_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(master_id)
            .join("todo.md");
        assert_eq!(fs::read_to_string(&plan_path).unwrap(), content);

        let got = get_by_id(master_id);
        assert_eq!(got["todo_md"], content);
        let list = list_all();
        assert_eq!(list.as_array().unwrap()[0]["todo_md"], content);
    });
}

#[test]
fn migrate_implicit_subs_write_failure_marks_migration_error() {
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
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
    with_todo_task_sandbox(|_| {
        let v = update_master_title("task_nonexistent_aaaaaaaaaaaaaaaa", "New");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

#[test]
fn update_master_fields_title_only_preserves_body() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs_and_todo("Old title", None, "# Body");
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().unwrap();

        let updated = update_master_fields(master_id, Some("  New title  "), None);
        assert_eq!(updated["_status"], 200);
        let task = master_from_value(&updated);
        assert_eq!(task["title"], "New title");
        assert_eq!(task["todo_md"], "# Body");

        let got = get_by_id(master_id);
        assert_eq!(got["title"], "New title");
        assert_eq!(got["todo_md"], "# Body");
    });
}

#[test]
fn update_master_fields_body_only_preserves_title() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs_and_todo("Keep title", None, "old");
        let master_id = created["master_task_id"].as_str().unwrap();

        let updated = update_master_fields(master_id, None, Some("## New body\n"));
        assert_eq!(updated["_status"], 200);
        let task = master_from_value(&updated);
        assert_eq!(task["title"], "Keep title");
        assert_eq!(task["todo_md"], "## New body\n");

        let got = get_by_id(master_id);
        assert_eq!(got["title"], "Keep title");
        assert_eq!(got["todo_md"], "## New body\n");
    });
}

#[test]
fn update_master_fields_both_title_and_body() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs_and_todo("A", None, "old");
        let master_id = created["master_task_id"].as_str().unwrap();

        let updated = update_master_fields(master_id, Some("B"), Some("new"));
        assert_eq!(updated["_status"], 200);
        let task = master_from_value(&updated);
        assert_eq!(task["title"], "B");
        assert_eq!(task["todo_md"], "new");
    });
}

#[test]
fn update_master_fields_empty_body_clears_todo_md() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs_and_todo("T", None, "has body");
        let master_id = created["master_task_id"].as_str().unwrap();

        let updated = update_master_fields(master_id, None, Some(""));
        assert_eq!(updated["_status"], 200);
        assert_eq!(master_from_value(&updated)["todo_md"], "");
        assert_eq!(get_by_id(master_id)["todo_md"], "");
    });
}

#[test]
fn update_master_fields_neither_field_returns_400() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Keep", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = update_master_fields(master_id, None, None);
        assert_eq!(v["_status"], 400);
        assert_eq!(v["error"], "Missing title or todo_md");
        assert_eq!(get_by_id(master_id)["title"], "Keep");
    });
}

#[test]
fn update_master_fields_blank_title_returns_400() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Keep me", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = update_master_fields(master_id, Some("   \t  "), None);
        assert_eq!(v["_status"], 400);
        assert_eq!(v["error"], "Missing title");
        assert_eq!(get_by_id(master_id)["title"], "Keep me");
    });
}

#[test]
fn update_master_fields_unknown_master_returns_404() {
    with_todo_task_sandbox(|_| {
        let v = update_master_fields("task_nonexistent_aaaaaaaaaaaaaaaa", Some("New"), None);
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

fn attachments_dir(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("attachments")
}

fn attachments_json_path(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("attachments.json")
}

fn load_attachments_file(wb: &Path, master_id: &str) -> AttachmentsFile {
    let text = fs::read_to_string(attachments_json_path(wb, master_id)).expect("attachments.json");
    serde_json::from_str(&text).expect("parse attachments.json")
}


fn write_attach_source(name: &str, content: impl AsRef<[u8]>) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "todo_attach_src_{}_{}",
        std::process::id(),
        n
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path.canonicalize().unwrap_or(path)
}

fn add_from(master_id: &str, name: &str, content: impl AsRef<[u8]>) -> serde_json::Value {
    let path = write_attach_source(name, content);
    add_attachment(master_id, path.to_str().unwrap())
}

fn save_from(
    master_id: &str,
    file_name: &str,
    source_name: &str,
    content: impl AsRef<[u8]>,
) -> serde_json::Value {
    let path = write_attach_source(source_name, content);
    save_attachment(master_id, file_name, path.to_str().unwrap())
}


#[test]
fn add_attachment_md_copies_and_writes_manifest() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Attach me", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let content = "# Notes\n\nhello";

        let v = add_from(master_id, "notes.md", content);
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Conflict", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let first = add_from(master_id, "notes.md", "one");
        assert_eq!(first["_status"], 201);
        assert_eq!(first["file_name"], "notes.md");

        let second = add_from(master_id, "notes.md", "two");
        assert_eq!(second["_status"], 201);
        assert_eq!(second["file_name"], "notes-1.md");
        assert_eq!(second["original_file_name"], "notes.md");

        let third = add_from(master_id, "notes.md", "three");
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Empty md", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = add_from(master_id, "empty.md", "");
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Upper ext", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = add_from(master_id, "NOTES.MD", "upper");
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Reject txt", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let v = add_from(master_id, "notes.txt", "nope");
        assert_eq!(v["_status"], 400);
        assert!(v.get("error").is_some());

        assert!(!attachments_dir(wb, master_id).exists());
        assert!(!attachments_json_path(wb, master_id).exists());
    });
}

#[test]
fn add_attachment_rejects_relative_and_disallowed_source_path() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Bad names", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let relative = add_attachment(master_id, "notes.md");
        assert_eq!(relative["_status"], 400);
        assert_eq!(relative["error"], "Invalid source_path");

        let missing = add_attachment(master_id, "/tmp/todo_attach_missing_no_such_file.md");
        assert_eq!(missing["_status"], 404);
        assert_eq!(missing["error"], "Source file not found");

        assert!(!attachments_dir(wb, master_id).exists());
        assert!(!attachments_json_path(wb, master_id).exists());
    });
}

#[test]

fn add_attachment_manifest_write_failure_rolls_back_copy() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Rollback", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let ok = add_from(master_id, "keep.md", "keep-me");
        assert_eq!(ok["_status"], 201);
        let before = load_attachments_file(wb, master_id);

        test_set_fail_add_attachment_manifest(true);
        let v = add_from(master_id, "new.md", "should-roll-back");
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
    with_todo_task_sandbox(|wb| {
        let v = add_from("task_nonexistent_aaaaaaaaaaaaaaaa", "notes.md", "x");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
        assert!(!wb
            .join("todo_tasks")
            .join("tasks")
            .join("task_nonexistent_aaaaaaaaaaaaaaaa")
            .exists());
    });
}

#[test]
fn add_attachment_unwritable_task_dir_fails_without_half_success() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Readonly dir", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);

        let mut perms = fs::metadata(&task_dir).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&task_dir, perms).unwrap();

        let v = add_from(master_id, "notes.md", "x");
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("List attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let a = add_from(master_id, "a.md", "one");
        let b = add_from(master_id, "b.md", "two");
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
    with_todo_task_sandbox(|wb| {
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
    with_todo_task_sandbox(|_| {
        let v = list_attachments("task_nonexistent_aaaaaaaaaaaaaaaa");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

#[test]
fn read_attachment_returns_manifest_file_content() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Read attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let content = "# Body\n\nline";

        let added = add_from(master_id, "notes.md", content);
        assert_eq!(added["_status"], 201);

        let v = read_attachment(master_id, "notes.md");
        assert_eq!(v["_status"], 200);
        assert_eq!(v["file_name"], "notes.md");
        assert_eq!(v["content"], content);
    });
}

#[test]
fn read_attachment_rejects_non_manifest_target() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Read reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        add_from(master_id, "listed.md", "ok");

        fs::create_dir_all(attachments_dir(wb, master_id)).unwrap();
        fs::write(attachments_dir(wb, master_id).join("orphan.md"), "ghost").unwrap();

        let v = read_attachment(master_id, "orphan.md");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn save_attachment_writes_content_without_touching_todo_md() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs_and_todo("Save attach", None, "# Plan body");
        let master_id = created["master_task_id"].as_str().unwrap();
        let plan_path = wb
            .join("todo_tasks")
            .join("tasks")
            .join(master_id)
            .join("todo.md");
        let plan_before = fs::read_to_string(&plan_path).unwrap();

        let added = add_from(master_id, "notes.md", "old");
        assert_eq!(added["_status"], 201);

        let v = save_from(master_id, "notes.md", "notes.md", "new content");
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("notes.md")).unwrap(),
            "new content"
        );
        assert_eq!(fs::read_to_string(&plan_path).unwrap(), plan_before);
        assert_eq!(read_todo_md(master_id)["todo_md"], "# Plan body");
    });
}

#[test]
fn save_attachment_allows_empty_content() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Save empty", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        add_from(master_id, "notes.md", "had text");

        let v = save_from(master_id, "notes.md", "notes.md", "");
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Save reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        add_from(master_id, "listed.md", "ok");

        fs::create_dir_all(attachments_dir(wb, master_id)).unwrap();
        fs::write(attachments_dir(wb, master_id).join("orphan.md"), "ghost").unwrap();

        let v = save_from(master_id, "orphan.md", "orphan.md", "overwrite");
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let added = add_from(master_id, "notes.md", "to-delete");
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete keeps others", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        assert_eq!(add_from(master_id, "keep-a.md", "a")["_status"], 201);
        assert_eq!(add_from(master_id, "drop.md", "drop-me")["_status"], 201);
        assert_eq!(add_from(master_id, "keep-b.md", "b")["_status"], 201);
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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert_eq!(add_from(master_id, "listed.md", "ok")["_status"], 201);

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
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete rollback", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        assert_eq!(add_from(master_id, "keep.md", "keep-me")["_status"], 201);
        assert_eq!(add_from(master_id, "drop.md", "drop-me")["_status"], 201);
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
    with_todo_task_sandbox(|_| {
        let v = delete_attachment("task_nonexistent_aaaaaaaaaaaaaaaa", "notes.md");
        assert_eq!(v["_status"], 404);
        assert_eq!(v["error"], "Task not found");
    });
}

#[test]
fn delete_master_cascades_attachments_dir_and_manifest() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete with attach", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let add = add_from(master_id, "notes.md", "# keep\n");
        assert_eq!(add["_status"], 201);

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
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
            &fs::read_to_string(wb.join("todo_tasks").join("index.json")).unwrap(),
        )
        .unwrap();
        assert!(index["tasks"].get(master_id).is_none());
    });
}

#[test]
fn delete_master_without_attachments_matches_existing_behavior() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete bare", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
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

// --- T15 AC regression locks (tech-doc VF AC1/AC3/AC6) ---

#[test]
fn ac15_stem_conflict_appends_suffix_and_keeps_originals() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("AC15 suffix", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        assert_eq!(add_from(master_id, "doc.md", "a")["_status"], 201);
        let second = add_from(master_id, "doc.md", "b");
        assert_eq!(second["_status"], 201);
        assert_eq!(second["file_name"], "doc-1.md");
        assert_eq!(second["original_file_name"], "doc.md");

        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("doc.md")).unwrap(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(attachments_dir(wb, master_id).join("doc-1.md")).unwrap(),
            "b"
        );
        let listed = list_attachments(master_id);
        assert_eq!(listed["_status"], 200);
        let names: Vec<&str> = listed["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["file_name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["doc.md", "doc-1.md"]);
    });
}

#[test]
fn ac15_add_manifest_failure_rolls_back_without_half_success() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("AC15 rollback", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert_eq!(add_from(master_id, "keep.md", "keep")["_status"], 201);
        let before = load_attachments_file(wb, master_id);

        test_set_fail_add_attachment_manifest(true);
        let v = add_from(master_id, "ghost.md", "ghost");
        assert_eq!(v["_status"], 500);
        assert!(!attachments_dir(wb, master_id).join("ghost.md").exists());
        assert_eq!(load_attachments_file(wb, master_id), before);
    });
}

#[test]
fn ac15_delete_clears_manifest_entry_and_file() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("AC15 dual clear", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert_eq!(add_from(master_id, "gone.md", "x")["_status"], 201);

        let v = delete_attachment(master_id, "gone.md");
        assert_eq!(v["_status"], 200);
        assert!(!attachments_dir(wb, master_id).join("gone.md").exists());
        assert!(load_attachments_file(wb, master_id)
            .attachments
            .iter()
            .all(|e| e.file_name != "gone.md"));
        assert_eq!(
            list_attachments(master_id)["attachments"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    });
}

#[test]
fn ac15_empty_list_and_non_md_reject() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("AC15 boundary", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let empty = list_attachments(master_id);
        assert_eq!(empty["_status"], 200);
        assert_eq!(empty["attachments"].as_array().unwrap().len(), 0);

        let rejected = add_from(master_id, "notes.txt", "nope");
        assert_eq!(rejected["_status"], 400);
        assert!(!attachments_dir(wb, master_id).exists());
        assert!(!attachments_json_path(wb, master_id).exists());
    });
}

fn comments_json_abs_path(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("comments.json")
}

#[test]
fn load_comments_missing_file_returns_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments missing file", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        assert!(!path.exists());

        let loaded = load_comments_file_unlocked(&path).expect("missing file is empty");
        assert!(loaded.comments.is_empty());
    });
}

#[test]
fn load_comments_empty_envelope_returns_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments empty envelope", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        fs::write(&path, r#"{ "comments": [] }"#).expect("write empty envelope");

        let loaded = load_comments_file_unlocked(&path).expect("empty envelope ok");
        assert!(loaded.comments.is_empty());
    });
}

#[test]
fn comments_file_round_trip_via_load_and_save() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments round-trip", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        let file = CommentsFile {
            comments: vec![CommentEntry {
                id: "cmt_abcdef012345".to_string(),
                body: "process note".to_string(),
                created_at: "2026-07-23T01:02:03+00:00".to_string(),
            }],
        };
        save_comments_file_unlocked(&path, &file).expect("save");
        assert!(path.is_file());

        let loaded = load_comments_file_unlocked(&path).expect("load");
        assert_eq!(loaded.comments.len(), 1);
        assert_eq!(loaded.comments[0].id, "cmt_abcdef012345");
        assert_eq!(loaded.comments[0].body, "process note");
        assert_eq!(loaded.comments[0].created_at, "2026-07-23T01:02:03+00:00");
    });
}

#[test]
fn mint_comment_id_and_created_at_follow_contract() {
    let id = mint_comment_id();
    assert!(id.starts_with("cmt_"), "id={id}");
    let hex = &id["cmt_".len()..];
    assert!(!hex.is_empty());
    assert!(
        hex.chars().all(|c| c.is_ascii_hexdigit()),
        "hex part must be hex: {hex}"
    );

    let created_at = now_comment_created_at();
    assert!(is_iso8601(&created_at), "created_at={created_at}");
}

#[test]
fn load_comments_bad_json_or_bare_array_is_explicit_error() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments bad json", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        fs::write(&path, "{not valid json").expect("write bad");
        let bad = load_comments_file_unlocked(&path);
        assert!(bad.is_err(), "bad JSON must error, not empty list");

        fs::write(
            &path,
            r#"[{"id":"cmt_x","body":"a","created_at":"2026-07-23T00:00:00+00:00"}]"#,
        )
        .expect("write bare array");
        let bare = load_comments_file_unlocked(&path);
        assert!(bare.is_err(), "bare array must error, not empty list");

        fs::write(&path, r#"{ "comments": "not-an-array" }"#).expect("write invalid envelope");
        let invalid = load_comments_file_unlocked(&path);
        assert!(invalid.is_err(), "invalid envelope must error");
    });
}

#[test]
fn validate_comment_body_rejects_empty_and_whitespace() {
    assert!(validate_comment_body("ok note").is_ok());
    assert!(validate_comment_body("").is_err());
    assert!(validate_comment_body("   ").is_err());
    assert!(validate_comment_body("\t\n").is_err());
}

#[test]
fn save_comments_file_does_not_enforce_count_cap() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments no count cap", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        let comments: Vec<CommentEntry> = (0..80)
            .map(|i| CommentEntry {
                id: format!("cmt_{i:012x}"),
                body: format!("body-{i}"),
                created_at: "2026-07-23T00:00:00+00:00".to_string(),
            })
            .collect();
        let file = CommentsFile { comments };
        save_comments_file_unlocked(&path, &file).expect("no count-cap on save");
        let loaded = load_comments_file_unlocked(&path).expect("load");
        assert_eq!(loaded.comments.len(), 80);
    });
}

#[test]
fn list_comments_missing_file_returns_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("List comments missing", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert!(!comments_json_abs_path(wb, master_id).exists());

        let v = list_comments(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["comments"].as_array().expect("comments array").len(), 0);
    });
}

#[test]
fn list_comments_orders_by_created_at_ascending() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("List comments order", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        let file = CommentsFile {
            comments: vec![
                CommentEntry {
                    id: "cmt_later000001".to_string(),
                    body: "second".to_string(),
                    created_at: "2026-07-23T02:00:00+00:00".to_string(),
                },
                CommentEntry {
                    id: "cmt_earlier0001".to_string(),
                    body: "first".to_string(),
                    created_at: "2026-07-23T01:00:00+00:00".to_string(),
                },
            ],
        };
        save_comments_file_unlocked(&path, &file).expect("seed");

        let v = list_comments(master_id);
        assert_eq!(v["_status"], 200);
        let items = v["comments"].as_array().expect("comments array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "cmt_earlier0001");
        assert_eq!(items[0]["body"], "first");
        assert_eq!(items[1]["id"], "cmt_later000001");
        assert_eq!(items[1]["body"], "second");
    });
}

#[test]
fn add_comment_writes_new_id_and_created_at_then_list_shows_it() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Add comment", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let added = add_comment(master_id, "process note");
        assert_eq!(added["_status"], 201);
        let id = added["id"].as_str().expect("id");
        assert!(id.starts_with("cmt_"), "id={id}");
        let created_at = added["created_at"].as_str().expect("created_at");
        assert!(is_iso8601(created_at), "created_at={created_at}");
        assert_eq!(added["body"], "process note");

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], id);
        assert_eq!(items[0]["body"], "process note");
        assert_eq!(items[0]["created_at"], created_at);
    });
}

#[test]
fn update_comment_changes_only_body_preserving_id_created_at_and_order() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Update comment", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        let file = CommentsFile {
            comments: vec![
                CommentEntry {
                    id: "cmt_a00000000001".to_string(),
                    body: "old-a".to_string(),
                    created_at: "2026-07-23T01:00:00+00:00".to_string(),
                },
                CommentEntry {
                    id: "cmt_b00000000002".to_string(),
                    body: "old-b".to_string(),
                    created_at: "2026-07-23T02:00:00+00:00".to_string(),
                },
            ],
        };
        save_comments_file_unlocked(&path, &file).expect("seed");

        let updated = update_comment(master_id, "cmt_a00000000001", "new-a");
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["id"], "cmt_a00000000001");
        assert_eq!(updated["body"], "new-a");
        assert_eq!(updated["created_at"], "2026-07-23T01:00:00+00:00");

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "cmt_a00000000001");
        assert_eq!(items[0]["body"], "new-a");
        assert_eq!(items[0]["created_at"], "2026-07-23T01:00:00+00:00");
        assert_eq!(items[1]["id"], "cmt_b00000000002");
        assert_eq!(items[1]["body"], "old-b");
    });
}

#[test]
fn delete_comment_hard_removes_id_from_list() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Delete comment", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let a = add_comment(master_id, "keep");
        let b = add_comment(master_id, "drop");
        assert_eq!(a["_status"], 201);
        assert_eq!(b["_status"], 201);
        let drop_id = b["id"].as_str().unwrap().to_string();

        let deleted = delete_comment(master_id, &drop_id);
        assert_eq!(deleted["_status"], 200);
        assert_eq!(deleted["ok"], true);

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], a["id"]);
        assert!(items.iter().all(|c| c["id"] != drop_id));
    });
}

#[test]
fn add_comment_repeatedly_is_not_rejected_for_count() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Add comment no cap", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        for i in 0..12 {
            let v = add_comment(master_id, &format!("note-{i}"));
            assert_eq!(v["_status"], 201, "add #{i} must not be rejected for count");
        }

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        assert_eq!(listed["comments"].as_array().expect("comments").len(), 12);
    });
}

#[test]
fn delete_master_cascades_comments_json_via_remove_dir_all() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete master comments cascade", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let added = add_comment(master_id, "will cascade away");
        assert_eq!(added["_status"], 201);

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        let comments_path = comments_json_abs_path(wb, master_id);
        assert!(task_dir.is_dir());
        assert!(comments_path.is_file());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert!(!task_dir.exists());
        assert!(!comments_path.exists());
    });
}

#[test]
fn add_and_update_comment_reject_empty_or_whitespace_body() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Empty body reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        for bad in ["", "   ", "\t\n"] {
            let add = add_comment(master_id, bad);
            assert_ne!(add["_status"], 201, "empty body add must fail");
            assert!(add.get("error").is_some(), "add must be explicit error");
        }
        assert!(!comments_json_abs_path(wb, master_id).exists());

        let ok = add_comment(master_id, "seed");
        assert_eq!(ok["_status"], 201);
        let id = ok["id"].as_str().unwrap();

        for bad in ["", "   ", "\t\n"] {
            let upd = update_comment(master_id, id, bad);
            assert_ne!(upd["_status"], 200, "empty body update must fail");
            assert!(upd.get("error").is_some(), "update must be explicit error");
        }

        let listed = list_comments(master_id);
        assert_eq!(listed["comments"][0]["body"], "seed");
    });
}

#[test]
fn update_and_delete_comment_unknown_id_is_explicit_error() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Unknown comment id", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let ok = add_comment(master_id, "exists");
        assert_eq!(ok["_status"], 201);

        let upd = update_comment(master_id, "cmt_doesnotexist", "x");
        assert_ne!(upd["_status"], 200);
        assert!(upd.get("error").is_some());

        let del = delete_comment(master_id, "cmt_doesnotexist");
        assert_ne!(del["_status"], 200);
        assert!(del.get("error").is_some());

        let listed = list_comments(master_id);
        assert_eq!(listed["comments"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn bad_comments_json_errors_on_list_and_writes_without_overwrite() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Bad comments json", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        let bad = "{not valid json";
        fs::write(&path, bad).expect("seed bad");

        let listed = list_comments(master_id);
        assert_ne!(listed["_status"], 200);
        assert!(listed.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);

        let add = add_comment(master_id, "should fail");
        assert_ne!(add["_status"], 201);
        assert!(add.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);

        let upd = update_comment(master_id, "cmt_x", "nope");
        assert_ne!(upd["_status"], 200);
        assert!(upd.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);

        let del = delete_comment(master_id, "cmt_x");
        assert_ne!(del["_status"], 200);
        assert!(del.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);
    });
}


// --- T5 AC regression locks (tech-doc T5 / AC1,AC4,AC5,AC9 / 不变量#2,#4,#5,#6) ---

#[test]
fn t5_add_list_orders_by_created_at_update_preserves_order_hard_delete_removes() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 CRUD order hard-delete", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        // Seed out-of-order created_at so list must sort ascending (AC2).
        let path = comments_json_abs_path(wb, master_id);
        let file = CommentsFile {
            comments: vec![
                CommentEntry {
                    id: "cmt_later000001".to_string(),
                    body: "second".to_string(),
                    created_at: "2026-07-23T02:00:00+00:00".to_string(),
                },
                CommentEntry {
                    id: "cmt_earlier0001".to_string(),
                    body: "first".to_string(),
                    created_at: "2026-07-23T01:00:00+00:00".to_string(),
                },
            ],
        };
        save_comments_file_unlocked(&path, &file).expect("seed");

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "cmt_earlier0001");
        assert_eq!(items[1]["id"], "cmt_later000001");

        // update preserves id/created_at/order (AC3).
        let updated = update_comment(master_id, "cmt_earlier0001", "first-edited");
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["id"], "cmt_earlier0001");
        assert_eq!(updated["created_at"], "2026-07-23T01:00:00+00:00");
        let after_update = list_comments(master_id);
        let after_items = after_update["comments"].as_array().unwrap();
        assert_eq!(after_items[0]["id"], "cmt_earlier0001");
        assert_eq!(after_items[0]["body"], "first-edited");
        assert_eq!(after_items[1]["id"], "cmt_later000001");

        // Hard-delete by id — intentional Spec↔Design deviation (P4 / 不变量#5):
        // ordinary single-entry hard delete; no soft-delete / version-audit path (AC4).
        let deleted = delete_comment(master_id, "cmt_later000001");
        assert_eq!(deleted["_status"], 200);
        assert_eq!(deleted["ok"], true);
        assert!(deleted.get("soft_deleted").is_none());
        assert!(deleted.get("deleted_at").is_none());
        assert!(deleted.get("audit").is_none());

        let after_delete = list_comments(master_id);
        let remain = after_delete["comments"].as_array().unwrap();
        assert_eq!(remain.len(), 1);
        assert_eq!(remain[0]["id"], "cmt_earlier0001");
        assert!(remain.iter().all(|c| c["id"] != "cmt_later000001"));

        // Disk must not keep a soft-delete tombstone for the removed id.
        let on_disk: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let disk_ids: Vec<&str> = on_disk["comments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert!(!disk_ids.contains(&"cmt_later000001"));
        for entry in on_disk["comments"].as_array().unwrap() {
            let obj = entry.as_object().unwrap();
            assert!(!obj.contains_key("deleted"));
            assert!(!obj.contains_key("deleted_at"));
            assert!(!obj.contains_key("is_deleted"));
            assert!(!obj.contains_key("version"));
            assert!(!obj.contains_key("updated_at"));
            assert_eq!(obj.len(), 3, "comment fields only id/body/created_at");
        }
    });
}

#[test]
fn t5_delete_master_cascades_comments_json_with_task_dir() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 master cascade comments", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert_eq!(add_comment(master_id, "cascade-me")["_status"], 201);

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        let comments_path = comments_json_abs_path(wb, master_id);
        assert!(task_dir.is_dir());
        assert!(comments_path.is_file());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert!(!task_dir.exists());
        assert!(!comments_path.exists(), "comments.json must vanish with task dir (AC5)");
    });
}

#[test]
fn t5_repeated_nonempty_add_is_not_rejected_for_count() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("T5 no count cap", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        // AC9 / 不变量#6: add must not refuse based on existing count.
        for i in 0..20 {
            let v = add_comment(master_id, &format!("t5-note-{i}"));
            assert_eq!(
                v["_status"], 201,
                "add #{i} must succeed; no count-cap rejection"
            );
        }
        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        assert_eq!(listed["comments"].as_array().unwrap().len(), 20);
    });
}

#[test]
fn t5_list_missing_file_and_empty_envelope_are_empty_not_error() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 empty list semantics", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        assert!(!path.exists());
        let missing = list_comments(master_id);
        assert_eq!(missing["_status"], 200);
        assert_eq!(missing["comments"].as_array().unwrap().len(), 0);

        fs::write(&path, r#"{"comments":[]}"#).unwrap();
        let empty = list_comments(master_id);
        assert_eq!(empty["_status"], 200);
        assert_eq!(empty["comments"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn t5_empty_or_whitespace_body_rejected_on_add_and_update() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 empty body", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        for bad in ["", "   ", "\t\n", " \t "] {
            let add = add_comment(master_id, bad);
            assert_eq!(add["_status"], 400, "empty/whitespace add must 400");
            assert!(add.get("error").is_some());
        }
        assert!(!comments_json_abs_path(wb, master_id).exists());

        let ok = add_comment(master_id, "seed-body");
        assert_eq!(ok["_status"], 201);
        let id = ok["id"].as_str().unwrap();
        for bad in ["", "   ", "\t\n"] {
            let upd = update_comment(master_id, id, bad);
            assert_eq!(upd["_status"], 400, "empty/whitespace update must 400");
            assert!(upd.get("error").is_some());
        }
        assert_eq!(list_comments(master_id)["comments"][0]["body"], "seed-body");
    });
}

#[test]
fn t5_bad_json_and_invalid_envelope_are_explicit_errors_not_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 bad json envelope", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        let cases = [
            "{not valid json",
            r#"[{"id":"cmt_x","body":"a","created_at":"2026-07-23T00:00:00+00:00"}]"#,
            r#"{"comments":"not-an-array"}"#,
            "", // empty file = corrupt, not empty-list success (AC1)
        ];
        for bad in cases {
            fs::write(&path, bad).unwrap();
            let listed = list_comments(master_id);
            assert_ne!(
                listed["_status"], 200,
                "corrupt comments.json must not succeed; payload={bad:?}"
            );
            assert!(listed.get("error").is_some());
            // Must not equal empty-list success semantics (缺文件/合法空 envelope).
            let disguised_empty = listed["_status"] == 200
                && listed
                    .get("comments")
                    .and_then(|c| c.as_array())
                    .map(|a| a.is_empty())
                    .unwrap_or(false);
            assert!(
                !disguised_empty,
                "bad JSON/envelope must not look like empty list"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), bad);
        }
    });
}

#[test]
fn t5_soft_delete_or_audit_fields_are_explicit_errors_not_accepted_schema() {
    // Spec「删除特判」偏差为有意：普通单条硬删除；无 soft-delete / 版本审计路径（不变量#5 / T5）。
    // Soft-delete-shaped or audit fields must be rejected as invalid envelope — not silently loaded.
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 reject soft-delete fields", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        fs::write(
            &path,
            r#"{
              "comments": [{
                "id": "cmt_soft0000001",
                "body": "should not load",
                "created_at": "2026-07-23T00:00:00+00:00",
                "deleted": true,
                "deleted_at": "2026-07-23T01:00:00+00:00"
              }]
            }"#,
        )
        .unwrap();

        let listed = list_comments(master_id);
        assert_ne!(
            listed["_status"], 200,
            "soft-delete fields must be explicit error, not accepted schema"
        );
        assert!(listed.get("error").is_some());
        let disguised_empty = listed["_status"] == 200
            && listed
                .get("comments")
                .and_then(|c| c.as_array())
                .map(|a| a.is_empty())
                .unwrap_or(false);
        assert!(!disguised_empty);

        // version-audit shaped field likewise rejected
        fs::write(
            &path,
            r#"{
              "comments": [{
                "id": "cmt_audit000001",
                "body": "should not load",
                "created_at": "2026-07-23T00:00:00+00:00",
                "version": 1
              }]
            }"#,
        )
        .unwrap();
        let listed_audit = list_comments(master_id);
        assert_ne!(listed_audit["_status"], 200);
        assert!(listed_audit.get("error").is_some());
    });
}

#[test]
fn plan_tasks_categories_path_is_ssot_under_todo_tasks() {
    with_todo_task_sandbox(|wb| {
        let path = paths::plan_tasks_categories_path().expect("path");
        assert_eq!(path, wb.join("todo_tasks").join("categories.json"));
        let sediment = paths::sediment_kb_categories_path().expect("sediment");
        assert_ne!(path, sediment);
    });
}

#[test]
fn load_categories_injects_default_when_registry_missing() {
    with_todo_task_sandbox(|wb| {
        let cats_path = wb.join("todo_tasks").join("categories.json");
        assert!(!cats_path.exists());
        let cats = load_categories().expect("load");
        assert!(
            cats.categories.iter().any(|c| {
                c.id == DEFAULT_CATEGORY_ID
                    && c.name == DEFAULT_CATEGORY_NAME
                    && c.is_default
            }),
            "missing registry must inject built-in 待分类"
        );
    });
}

#[test]
fn ensure_default_category_persists_registry_via_paths() {
    with_todo_task_sandbox(|wb| {
        let cats = ensure_default_category().expect("ensure");
        assert!(cats
            .categories
            .iter()
            .any(|c| c.id == DEFAULT_CATEGORY_ID && c.is_default));
        let path = paths::plan_tasks_categories_path().expect("path");
        assert_eq!(path, wb.join("todo_tasks").join("categories.json"));
        assert!(path.is_file(), "ensure may create default registry file");
        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains(DEFAULT_CATEGORY_ID));
        assert!(text.contains(DEFAULT_CATEGORY_NAME));
    });
}

#[test]
fn missing_categories_registry_does_not_block_todo_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("No cats yet", None);
        assert_eq!(created["_status"], 201);
        let cats_path = wb.join("todo_tasks").join("categories.json");
        if cats_path.is_file() {
            fs::remove_file(&cats_path).expect("remove cats");
        }
        let listed = list_all();
        let arr = listed.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "No cats yet");
    });
}

#[test]
fn todo_missing_category_id_reads_as_default_without_rewriting_disk() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Legacy todo", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        let index_path = wb.join("todo_tasks").join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["tasks"][master_id]
            .as_object_mut()
            .unwrap()
            .remove("category_id");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let before = fs::read_to_string(&index_path).unwrap();
        assert!(!before.contains("\"category_id\""));

        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], DEFAULT_CATEGORY_ID);

        let after = fs::read_to_string(&index_path).unwrap();
        assert_eq!(before, after, "compat read must not rewrite disk");
    });
}

#[test]
fn todo_orphan_category_id_reads_as_default_without_rewriting_disk() {
    with_todo_task_sandbox(|wb| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_subs("Orphan cat", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        let index_path = wb.join("todo_tasks").join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["tasks"][master_id]["category_id"] = serde_json::json!("cat_missing_zzz");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let before = fs::read_to_string(&index_path).unwrap();
        assert!(before.contains("cat_missing_zzz"));

        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], DEFAULT_CATEGORY_ID);

        let after = fs::read_to_string(&index_path).unwrap();
        assert_eq!(before, after, "orphan compat read must not rewrite disk");
        assert!(after.contains("cat_missing_zzz"));
    });
}

#[test]
fn todo_valid_category_id_roundtrips_on_read() {
    with_todo_task_sandbox(|wb| {
        ensure_default_category().expect("ensure");
        let cats_path = paths::plan_tasks_categories_path().expect("path");
        let mut cats = load_categories().expect("load");
        cats.categories
            .push(crate::services::todo_task::types::Category {
                id: "cat_work".to_string(),
                name: "Work".to_string(),
                is_default: false,
            });
        fs::write(&cats_path, serde_json::to_string_pretty(&cats).unwrap()).unwrap();

        let created = create_master_with_subs("Work todo", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        let index_path = wb.join("todo_tasks").join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["tasks"][master_id]["category_id"] = serde_json::json!("cat_work");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], "cat_work");
        let listed = list_all();
        assert_eq!(listed.as_array().unwrap()[0]["category_id"], "cat_work");
    });
}

// --- t2: Host category rules, migration, create/set category ---

#[test]
fn create_without_category_writes_default_uncategorized_id() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_category("Default cat", None, "", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        assert_eq!(created["task"]["category_id"], DEFAULT_CATEGORY_ID);

        let index_path = wb.join("todo_tasks").join("index.json");
        let index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        assert_eq!(
            index["tasks"][master_id]["category_id"],
            DEFAULT_CATEGORY_ID,
            "create omit must persist default 待分类 id"
        );
    });
}

#[test]
fn create_with_valid_category_id_assigns_that_category() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let created_cat = create_todo_category("Work");
        assert_eq!(created_cat["_status"], 201);
        let cat_id = created_cat["category_id"].as_str().expect("cat id").to_string();

        let created = create_master_with_category("In work", None, "", Some(&cat_id));
        assert_eq!(created["_status"], 201);
        assert_eq!(created["task"]["category_id"], cat_id);
        let master_id = created["master_task_id"].as_str().expect("id");
        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], cat_id);
    });
}

#[test]
fn create_with_unknown_category_id_rejects_without_fallback() {
    with_todo_task_sandbox(|wb| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_category("Bad cat", None, "", Some("cat_missing_zzz"));
        assert!(
            created["_status"].as_u64().unwrap_or(0) >= 400,
            "unknown category_id must reject"
        );
        assert!(created.get("error").is_some());

        let index_path = wb.join("todo_tasks").join("index.json");
        if index_path.is_file() {
            let index: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
            let tasks = index["tasks"].as_object().expect("tasks");
            assert!(
                tasks.is_empty(),
                "rejected create must not persist a todo"
            );
        }
    });
}

#[test]
fn set_master_category_valid_id_updates_membership() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let cat = create_todo_category("Later");
        let cat_id = cat["category_id"].as_str().expect("id").to_string();
        let created = create_master_with_category("Move me", None, "", None);
        let master_id = created["master_task_id"].as_str().expect("id");

        let updated = set_master_category(master_id, &cat_id);
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["task"]["category_id"], cat_id);
        assert_eq!(get_by_id(master_id)["category_id"], cat_id);
    });
}

#[test]
fn set_master_category_unknown_id_rejects_without_fallback() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_category("Keep default", None, "", None);
        let master_id = created["master_task_id"].as_str().expect("id");

        let updated = set_master_category(master_id, "cat_does_not_exist");
        assert!(updated["_status"].as_u64().unwrap_or(0) >= 400);
        assert!(updated.get("error").is_some());
        assert_eq!(
            get_by_id(master_id)["category_id"],
            DEFAULT_CATEGORY_ID,
            "failed set must not fall back by rewriting"
        );
    });
}

#[test]
fn list_todo_categories_includes_default() {
    with_todo_task_sandbox(|_| {
        let listed = list_todo_categories();
        assert_eq!(listed["_status"], 200);
        let cats = listed["categories"].as_array().expect("categories");
        assert!(
            cats.iter().any(|c| {
                c["id"] == DEFAULT_CATEGORY_ID
                    && c["name"] == DEFAULT_CATEGORY_NAME
                    && c["is_default"] == true
            }),
            "list must include built-in 待分类"
        );
    });
}

#[test]
fn create_todo_category_persists_and_appears_in_list() {
    with_todo_task_sandbox(|_| {
        let created = create_todo_category("Research");
        assert_eq!(created["_status"], 201);
        let cat_id = created["category_id"].as_str().expect("id");
        assert!(!cat_id.is_empty());
        assert_eq!(created["category"]["name"], "Research");
        assert_eq!(created["category"]["is_default"], false);

        let listed = list_todo_categories();
        assert!(listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id && c["name"] == "Research"));
    });
}

#[test]
fn delete_empty_category_succeeds_and_disappears_from_list() {
    with_todo_task_sandbox(|_| {
        let created = create_todo_category("Ephemeral");
        let cat_id = created["category_id"].as_str().expect("id").to_string();
        let deleted = delete_todo_category(&cat_id);
        assert_eq!(deleted["_status"], 200);
        assert_eq!(deleted["ok"], true);

        let listed = list_todo_categories();
        assert!(!listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id));
    });
}

#[test]
fn delete_default_category_always_fails() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let deleted = delete_todo_category(DEFAULT_CATEGORY_ID);
        assert!(deleted["_status"].as_u64().unwrap_or(0) >= 400);
        assert!(deleted.get("error").is_some());

        let listed = list_todo_categories();
        assert!(listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == DEFAULT_CATEGORY_ID));
    });
}

#[test]
fn delete_non_empty_category_fails_without_reassigning_members() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let cat = create_todo_category("Occupied");
        let cat_id = cat["category_id"].as_str().expect("id").to_string();
        let created = create_master_with_category("Member", None, "", Some(&cat_id));
        let master_id = created["master_task_id"].as_str().expect("id");

        let deleted = delete_todo_category(&cat_id);
        assert!(deleted["_status"].as_u64().unwrap_or(0) >= 400);
        assert!(deleted.get("error").is_some());

        assert_eq!(get_by_id(master_id)["category_id"], cat_id);
        let listed = list_todo_categories();
        assert!(listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id));
    });
}

#[test]
fn migrate_todos_default_category_stamps_missing_ids() {
    with_todo_task_sandbox(|wb| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_subs("Legacy stamp", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        let index_path = wb.join("todo_tasks").join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["tasks"][master_id]
            .as_object_mut()
            .unwrap()
            .remove("category_id");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let report = migrate_todos_default_category();
        assert_eq!(report["_status"], 200);
        let updated = report["updated"].as_u64().unwrap_or(0);
        assert!(updated >= 1, "migration must update at least one todo");

        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        assert_eq!(
            after["tasks"][master_id]["category_id"],
            DEFAULT_CATEGORY_ID,
            "migration and create-default must share the same id"
        );
    });
}

#[test]
fn create_default_and_migration_share_same_default_category_id() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_category("Same default", None, "", None);
        let create_id = created["task"]["category_id"].as_str().expect("id");

        let master_id = created["master_task_id"].as_str().expect("id");
        let index_path = wb.join("todo_tasks").join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["tasks"][master_id]
            .as_object_mut()
            .unwrap()
            .remove("category_id");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let report = migrate_todos_default_category();
        assert_eq!(report["_status"], 200);
        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        assert_eq!(after["tasks"][master_id]["category_id"], create_id);
        assert_eq!(create_id, DEFAULT_CATEGORY_ID);
    });
}

/// T5 / SK-4 ops harness: default「待分类」→ temp category via set_master_category
/// (same Host path as MCP `update_todo_task` category_id). Optional evidence dump:
/// `T5_OPS_EVIDENCE_OUT=/abs/path.json`.
#[test]
fn t5_independent_reclassify_ops_demo() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_category("T5 reclass sample", None, "", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        assert_eq!(
            created["task"]["category_id"],
            DEFAULT_CATEGORY_ID,
            "sample starts in 待分类"
        );

        let cat = create_todo_category("T5 Ops Target");
        assert_eq!(cat["_status"], 201);
        let target_id = cat["category_id"].as_str().expect("id").to_string();
        let target_name = cat["category"]["name"].as_str().expect("name").to_string();

        let moved = set_master_category(&master_id, &target_id);
        assert_eq!(moved["_status"], 200);
        assert_eq!(moved["task"]["category_id"], target_id);
        assert_eq!(get_by_id(&master_id)["category_id"], target_id);

        let rejected = set_master_category(&master_id, "cat_invalid_t5_ops");
        assert!(rejected["_status"].as_u64().unwrap_or(0) >= 400);
        assert_eq!(
            get_by_id(&master_id)["category_id"],
            target_id,
            "invalid category_id must not rewrite disk"
        );

        let restored = set_master_category(&master_id, DEFAULT_CATEGORY_ID);
        assert_eq!(restored["_status"], 200);
        assert_eq!(get_by_id(&master_id)["category_id"], DEFAULT_CATEGORY_ID);

        let moved_again = set_master_category(&master_id, &target_id);
        assert_eq!(moved_again["_status"], 200);
        assert_eq!(get_by_id(&master_id)["category_id"], target_id);

        if let Ok(out) = std::env::var("T5_OPS_EVIDENCE_OUT") {
            let evidence = serde_json::json!({
                "task_id": "t5",
                "recorded_at": chrono::Utc::now().to_rfc3339(),
                "harness": "cargo test --lib t5_independent_reclassify_ops_demo (Host set_master_category; MCP update_todo_task category_id proxy)",
                "live_mcp_note": "IDE MCP schema at ops time lacked category_id; verified via worktree Host unit harness",
                "default_category": {
                    "id": DEFAULT_CATEGORY_ID,
                    "name": DEFAULT_CATEGORY_NAME,
                },
                "samples": [{
                    "master_task_id": master_id,
                    "title": "T5 reclass sample",
                    "from_category_id": DEFAULT_CATEGORY_ID,
                    "to_category_id": target_id,
                    "to_category_name": target_name,
                    "via": "set_master_category",
                    "status": "moved",
                }],
                "reversible": true,
                "invalid_category_rejected": true,
                "no_mcp_category_crud": true,
            });
            let path = Path::new(&out);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("evidence parent");
            }
            fs::write(
                path,
                serde_json::to_string_pretty(&evidence).expect("serialize evidence"),
            )
            .expect("write evidence");
        }
    });
}
