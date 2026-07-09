use std::fs;

use serde_json::json;

use crate::commands::plan_task::{
    add_plan_sub_json, create_plan_task_json, delete_plan_sub_json, delete_plan_task_json,
    get_plan_tasks_json,
};
use crate::services::plan_task::{create_master_with_subs, list_all};
use crate::test_support::TestSandbox;

fn master_from_invoke(v: &serde_json::Value) -> &serde_json::Value {
    v.get("task").expect("task field")
}

fn with_commands_plan_test<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    f();
}

fn assert_master_task_shape(task: &serde_json::Value) {
    assert!(task.get("master_task_id").and_then(|v| v.as_str()).is_some());
    assert!(task.get("title").and_then(|v| v.as_str()).is_some());
    let status = task.get("status").and_then(|v| v.as_str()).expect("status");
    assert!(status == "incomplete" || status == "complete");
    assert!(task.get("created_at").and_then(|v| v.as_str()).is_some());
    let subs = task["sub_tasks"].as_array().expect("sub_tasks");
    for sub in subs {
        assert!(sub.get("sub_task_id").and_then(|v| v.as_str()).is_some());
        assert!(sub.get("status").and_then(|v| v.as_str()).is_some());
        assert!(sub.get("implicit").map(|v| v.is_boolean()).unwrap_or(false));
        sub["linked_archive_ids"].as_array().expect("linked_archive_ids");
    }
}

#[test]
fn get_plan_tasks_json_empty_returns_empty_array() {
    with_commands_plan_test(|| {
        let listed = get_plan_tasks_json().expect("empty list");
        assert_eq!(listed.as_array().expect("array").len(), 0);
    });
}

#[test]
fn get_plan_tasks_json_returns_desc_sorted_array() {
    with_commands_plan_test(|| {
        let created = create_master_with_subs("Plan A", None);
        assert!(created.get("master_task_id").is_some());
        let listed = get_plan_tasks_json().expect("list");
        let arr = listed.as_array().expect("array");
        assert!(!arr.is_empty());
        assert_eq!(list_all(), listed);
    });
}

#[test]
fn get_plan_tasks_json_v2_empty_sub_tasks_shape() {
    with_commands_plan_test(|| {
        create_master_with_subs("Empty UI", None);
        let listed = get_plan_tasks_json().expect("list");
        let task = &listed.as_array().expect("array")[0];
        assert_master_task_shape(task);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
        assert_eq!(task["status"], "incomplete");
    });
}

#[test]
fn get_plan_tasks_json_v2_multi_sub_shape() {
    with_commands_plan_test(|| {
        create_master_with_subs("Multi UI", Some(&["A", "B"]));
        let listed = get_plan_tasks_json().expect("list");
        let task = &listed.as_array().expect("array")[0];
        assert_master_task_shape(task);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0]["implicit"], false);
        assert_eq!(subs[1]["implicit"], false);
    });
}

#[test]
fn get_plan_tasks_json_sorts_by_created_at_desc() {
    with_commands_plan_test(|| {
        create_master_with_subs("Older", None);
        std::thread::sleep(std::time::Duration::from_millis(5));
        create_master_with_subs("Newer", None);
        let listed = get_plan_tasks_json().expect("list");
        let arr = listed.as_array().expect("array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["title"], "Newer");
        assert_eq!(arr[1]["title"], "Older");
    });
}

#[test]
fn get_plan_tasks_json_corrupt_v2_storage_returns_err() {
    with_commands_plan_test(|| {
        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let master_id = "task_corrupt_cmd";
        let plan_tasks_dir = wb.join("plan_tasks");
        fs::create_dir_all(plan_tasks_dir.join("tasks").join(master_id)).expect("mkdir");
        let index = json!({
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
            plan_tasks_dir
                .join("tasks")
                .join(master_id)
                .join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let listed = get_plan_tasks_json().expect("list with migration_error");
        let arr = listed.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["master_task_id"], master_id);
        assert_eq!(arr[0]["migration_error"], true);
    });
}

#[test]
fn create_plan_task_json_title_only_creates_empty_sub_tasks() {
    with_commands_plan_test(|| {
        let v = create_plan_task_json("Empty cmd", None).expect("create");
        assert!(v.get("_status").is_none());
        assert!(v.get("master_task_id").and_then(|x| x.as_str()).is_some());
        assert!(v.get("sub_task_id").is_none());
        let task = master_from_invoke(&v);
        assert_master_task_shape(task);
        assert_eq!(task["title"], "Empty cmd");
        assert_eq!(task["status"], "incomplete");
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
    });
}

#[test]
fn create_plan_task_json_explicit_subs_strips_status() {
    with_commands_plan_test(|| {
        let v = create_plan_task_json("Multi cmd", Some(&["A", "B"])).expect("create");
        assert!(v.get("_status").is_none());
        let task = master_from_invoke(&v);
        assert_master_task_shape(task);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0]["implicit"], false);
        assert_eq!(subs[1]["implicit"], false);
        assert_eq!(subs[0]["title"], "A");
        assert_eq!(subs[1]["title"], "B");
    });
}

#[test]
fn create_plan_task_json_empty_title_returns_400_class() {
    with_commands_plan_test(|| {
        let v = create_plan_task_json("", None).expect("invoke");
        assert_eq!(v["error"], "Missing title");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn delete_plan_task_json_removes_master_from_list() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Delete me", None).expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let deleted = delete_plan_task_json(&master_id).expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed = get_plan_tasks_json().expect("list");
        let found = listed
            .as_array()
            .expect("array")
            .iter()
            .any(|t| t["master_task_id"] == master_id);
        assert!(!found);
    });
}

#[test]
fn add_plan_sub_json_appends_and_returns_updated_master() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Add sub", Some(&["A"])).expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let before_len = master_from_invoke(&created)["sub_tasks"]
            .as_array()
            .expect("subs")
            .len();

        let added = add_plan_sub_json(&master_id, "B").expect("add");
        assert!(added.get("_status").is_none());
        let task = master_from_invoke(&added);
        let subs = task["sub_tasks"].as_array().expect("subs");
        assert_eq!(subs.len(), before_len + 1);
        assert_eq!(subs[before_len]["title"], "B");
        assert_eq!(subs[before_len]["implicit"], false);
    });
}

#[test]
fn delete_plan_sub_json_keeps_remaining_when_not_last() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Del sub", Some(&["A", "B"])).expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        let deleted = delete_plan_sub_json(&master_id, &sub_a).expect("delete sub");
        assert!(deleted.get("_status").is_none());
        let task = master_from_invoke(&deleted);
        let subs = task["sub_tasks"].as_array().expect("subs");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["title"], "B");
    });
}

#[test]
fn delete_plan_sub_json_last_sub_allows_empty_sub_tasks() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Last sub", Some(&["Only"]))
            .expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let last_sub = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub")
            .to_string();

        let deleted = delete_plan_sub_json(&master_id, &last_sub).expect("invoke");
        assert!(deleted.get("_status").is_none());
        let task = master_from_invoke(&deleted);
        assert_eq!(task["status"], "incomplete");
        assert!(task["sub_tasks"].as_array().expect("subs").is_empty());

        let listed = get_plan_tasks_json().expect("list");
        let task = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("still present");
        assert!(task["sub_tasks"].as_array().expect("subs").is_empty());
    });
}

#[test]
fn delete_plan_task_json_unknown_id_returns_404_class() {
    with_commands_plan_test(|| {
        let v = delete_plan_task_json("task_nonexistent_aaaaaaaaaaaaaaaa")
            .expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}
