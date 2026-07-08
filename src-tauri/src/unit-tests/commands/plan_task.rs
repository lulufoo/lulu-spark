use std::fs;

use serde_json::json;

use crate::commands::plan_task::get_plan_tasks_json;
use crate::services::plan_task::{create_master_with_subs, list_all};
use crate::test_support::TestSandbox;

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
    assert!(!subs.is_empty());
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
fn get_plan_tasks_json_v2_implicit_sub_shape() {
    with_commands_plan_test(|| {
        create_master_with_subs("Implicit UI", None);
        let listed = get_plan_tasks_json().expect("list");
        let task = &listed.as_array().expect("array")[0];
        assert_master_task_shape(task);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["implicit"], true);
        assert_eq!(subs[0]["status"], "incomplete");
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

        let err = get_plan_tasks_json().expect_err("corrupt storage");
        assert_eq!(err, "Invalid plan_tasks storage");
    });
}
