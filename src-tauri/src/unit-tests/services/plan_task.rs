use super::*;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::config::paths;
use crate::config::settings::{self, default_cache_dir};
use crate::test_support::TestSandbox;

fn with_plan_task_sandbox<F: FnOnce(&Path)>(f: F) {
    crate::test_support::with_config_test_serial(|| {
        let _sandbox = TestSandbox::new();
        let wb = paths::workbench_knowledge_root().expect("workbench root");
        f(&wb);
    });
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
