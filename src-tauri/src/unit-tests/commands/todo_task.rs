use std::fs;

use serde_json::json;

use crate::commands::todo_task::{
    abandon_todo_sub_json, add_todo_attachment, add_todo_attachment_json, add_todo_comment,
    add_todo_comment_json, add_todo_sub, add_todo_sub_json, complete_todo, complete_todo_json,
    create_todo_category_json, create_todo_task_json, delete_todo_attachment,
    delete_todo_attachment_json, delete_todo_category_json, delete_todo_comment,
    delete_todo_comment_json, delete_todo_sub_json, delete_todo_task_json, get_todo_tasks_json,
    list_todo_attachments, list_todo_attachments_json, list_todo_categories_json,
    list_todo_comments, list_todo_comments_json, read_todo_attachment, read_todo_attachment_json,
    read_todo_md_json, save_todo_attachment, save_todo_attachment_json, set_todo_category_json,
    stage_todo_attachment_source, update_todo_comment, update_todo_comment_json,
    update_todo_master_title_json, update_todo_md_json, update_todo_sub, update_todo_sub_json,
};
use crate::services::todo_task::{
    create_master_with_subs, list_all, test_reset_all_injection_flags, test_run_write_task_batch,
    test_set_fail_batch_todo_md,
};
use crate::services::todo_task::types::{IndexEntry, MasterTaskStatus, SubTasksFile, DEFAULT_CATEGORY_ID};
use crate::test_support::TestSandbox;

fn master_from_invoke(v: &serde_json::Value) -> &serde_json::Value {
    v.get("task").expect("task field")
}

fn with_commands_todo_test<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    test_reset_all_injection_flags();
    f();
}

fn assert_master_task_shape(task: &serde_json::Value) {
    assert!(task.get("master_task_id").and_then(|v| v.as_str()).is_some());
    assert!(task.get("title").and_then(|v| v.as_str()).is_some());
    let status = task.get("status").and_then(|v| v.as_str()).expect("status");
    assert!(
        super::TODO_TASK_MASTER_STATUS_WIRE.contains(&status),
        "locked read exit status must be tri-state wire value, got {status}"
    );
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
fn get_todo_tasks_json_empty_returns_empty_array() {
    with_commands_todo_test(|| {
        let listed = get_todo_tasks_json().expect("empty list");
        assert_eq!(listed.as_array().expect("array").len(), 0);
    });
}

#[test]
fn get_todo_tasks_json_returns_desc_sorted_array() {
    with_commands_todo_test(|| {
        let created = create_master_with_subs("Plan A", None);
        assert!(created.get("master_task_id").is_some());
        let listed = get_todo_tasks_json().expect("list");
        let arr = listed.as_array().expect("array");
        assert!(!arr.is_empty());
        assert_eq!(list_all(), listed);
    });
}

#[test]
fn get_todo_tasks_json_v2_empty_sub_tasks_shape() {
    with_commands_todo_test(|| {
        create_master_with_subs("Empty UI", None);
        let listed = get_todo_tasks_json().expect("list");
        let task = &listed.as_array().expect("array")[0];
        assert_master_task_shape(task);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert!(subs.is_empty());
        assert_eq!(task["status"], "incomplete");
    });
}

#[test]
fn get_todo_tasks_json_v2_multi_sub_shape() {
    with_commands_todo_test(|| {
        create_master_with_subs("Multi UI", Some(&["A", "B"]));
        let listed = get_todo_tasks_json().expect("list");
        let task = &listed.as_array().expect("array")[0];
        assert_master_task_shape(task);
        let subs = task["sub_tasks"].as_array().expect("sub_tasks");
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0]["implicit"], false);
        assert_eq!(subs[1]["implicit"], false);
    });
}

#[test]
fn get_todo_tasks_json_sorts_by_created_at_desc() {
    with_commands_todo_test(|| {
        create_master_with_subs("Older", None);
        std::thread::sleep(std::time::Duration::from_millis(5));
        create_master_with_subs("Newer", None);
        let listed = get_todo_tasks_json().expect("list");
        let arr = listed.as_array().expect("array");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["title"], "Newer");
        assert_eq!(arr[1]["title"], "Older");
    });
}

#[test]
fn get_todo_tasks_json_corrupt_v2_storage_returns_err() {
    with_commands_todo_test(|| {
        let wb = crate::config::paths::workbench_knowledge_root().expect("wb");
        let master_id = "task_corrupt_cmd";
        let todo_tasks_dir = wb.join("todo_tasks");
        fs::create_dir_all(todo_tasks_dir.join("tasks").join(master_id)).expect("mkdir");
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
            todo_tasks_dir.join("index.json"),
            serde_json::to_string_pretty(&index).expect("serialize"),
        )
        .expect("write index");
        fs::write(
            todo_tasks_dir
                .join("tasks")
                .join(master_id)
                .join("sub_tasks.json"),
            "{not valid json",
        )
        .expect("write corrupt sub_tasks");

        let listed = get_todo_tasks_json().expect("list with migration_error");
        let arr = listed.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["master_task_id"], master_id);
        assert_eq!(arr[0]["migration_error"], true);
    });
}

#[test]
fn create_todo_task_json_title_only_creates_empty_sub_tasks() {
    with_commands_todo_test(|| {
        let v = create_todo_task_json("Empty cmd", None, "").expect("create");
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
fn create_todo_task_json_explicit_subs_strips_status() {
    with_commands_todo_test(|| {
        let v = create_todo_task_json("Multi cmd", Some(&["A", "B"]), "").expect("create");
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
fn create_todo_task_json_empty_title_returns_400_class() {
    with_commands_todo_test(|| {
        let v = create_todo_task_json("", None, "").expect("invoke");
        assert_eq!(v["error"], "Missing title");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn delete_todo_task_json_removes_master_from_list() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Delete me", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let deleted = delete_todo_task_json(&master_id).expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed = get_todo_tasks_json().expect("list");
        let found = listed
            .as_array()
            .expect("array")
            .iter()
            .any(|t| t["master_task_id"] == master_id);
        assert!(!found);
    });
}

#[test]
fn add_todo_sub_json_appends_and_returns_updated_master() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Add sub", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let before_len = master_from_invoke(&created)["sub_tasks"]
            .as_array()
            .expect("subs")
            .len();

        let added = add_todo_sub_json(&master_id, "B", None).expect("add");
        assert!(added.get("_status").is_none());
        let task = master_from_invoke(&added);
        let subs = task["sub_tasks"].as_array().expect("subs");
        assert_eq!(subs.len(), before_len + 1);
        assert_eq!(subs[before_len]["title"], "B");
        assert_eq!(subs[before_len]["implicit"], false);
        assert!(
            subs[before_len].get("content").is_none() || subs[before_len]["content"].is_null()
        );
    });
}

#[test]
fn add_todo_sub_json_with_optional_content_persists() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Add sub content", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let added =
            add_todo_sub_json(&master_id, "With body", Some("host content")).expect("add");
        assert!(added.get("_status").is_none());
        let task = master_from_invoke(&added);
        let subs = task["sub_tasks"].as_array().expect("subs");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["title"], "With body");
        assert_eq!(subs[0]["content"], "host content");

        let listed = get_todo_tasks_json().expect("list");
        let found = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("listed");
        assert_eq!(found["sub_tasks"][0]["content"], "host content");
    });
}

#[test]
fn update_todo_sub_json_title_only_leaves_content_unchanged() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Rename keep content", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let added =
            add_todo_sub_json(&master_id, "Old title", Some("keep me")).expect("add");
        let sub_id = master_from_invoke(&added)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub")
            .to_string();

        let updated =
            update_todo_sub_json(&master_id, &sub_id, "New title", None).expect("update");
        assert!(updated.get("_status").is_none());
        let task = master_from_invoke(&updated);
        assert_eq!(task["sub_tasks"][0]["title"], "New title");
        assert_eq!(task["sub_tasks"][0]["content"], "keep me");
    });
}

#[test]
fn update_todo_sub_json_sets_and_clears_optional_content() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Update content", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let added = add_todo_sub_json(&master_id, "Sub", Some("v1")).expect("add");
        let sub_id = master_from_invoke(&added)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub")
            .to_string();

        let set = update_todo_sub_json(&master_id, &sub_id, "Sub", Some("v2")).expect("set");
        assert!(set.get("_status").is_none());
        assert_eq!(master_from_invoke(&set)["sub_tasks"][0]["content"], "v2");

        let cleared =
            update_todo_sub_json(&master_id, &sub_id, "Sub", Some("")).expect("clear");
        assert!(cleared.get("_status").is_none());
        let sub = &master_from_invoke(&cleared)["sub_tasks"][0];
        assert!(sub.get("content").is_none() || sub["content"].is_null() || sub["content"] == "");
    });
}

#[test]
fn update_todo_sub_json_empty_title_returns_400_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Update blank title", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_id = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub")
            .to_string();

        for title in ["", "   "] {
            let v = update_todo_sub_json(&master_id, &sub_id, title, Some("ignored"))
                .expect("invoke");
            assert_eq!(v["error"], "Missing title");
            assert_eq!(v["_status"], 400);
        }
    });
}

#[test]
fn update_todo_sub_json_unknown_sub_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Update 404", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = update_todo_sub_json(master_id, "task_missing_sub_01", "New", None).expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn todo_sub_command_symbols_accept_optional_content() {
    // Smoke: async command symbols exist with optional content parameter surface.
    let _ = add_todo_sub;
    let _ = update_todo_sub;
}

#[test]
fn delete_todo_sub_json_keeps_remaining_when_not_last() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Del sub", Some(&["A", "B"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        let deleted = delete_todo_sub_json(&master_id, &sub_a).expect("delete sub");
        assert!(deleted.get("_status").is_none());
        let task = master_from_invoke(&deleted);
        let subs = task["sub_tasks"].as_array().expect("subs");
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["title"], "B");
    });
}

#[test]
fn delete_todo_sub_json_last_sub_allows_empty_sub_tasks() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Last sub", Some(&["Only"]), "")
            .expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let last_sub = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub")
            .to_string();

        let deleted = delete_todo_sub_json(&master_id, &last_sub).expect("invoke");
        assert!(deleted.get("_status").is_none());
        let task = master_from_invoke(&deleted);
        assert_eq!(task["status"], "incomplete");
        assert!(task["sub_tasks"].as_array().expect("subs").is_empty());

        let listed = get_todo_tasks_json().expect("list");
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
fn delete_todo_task_json_unknown_id_returns_404_class() {
    with_commands_todo_test(|| {
        let v = delete_todo_task_json("task_nonexistent_aaaaaaaaaaaaaaaa")
            .expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn read_todo_md_json_returns_todo_md_without_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Plan md cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let read = read_todo_md_json(master_id).expect("read");
        assert!(read.get("_status").is_none());
        assert_eq!(read["todo_md"], "");
    });
}

#[test]
fn read_todo_md_json_missing_id_returns_400_class() {
    with_commands_todo_test(|| {
        let v = read_todo_md_json("").expect("invoke");
        assert_eq!(v["error"], "Missing id");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn read_todo_md_json_unknown_master_returns_404_class() {
    with_commands_todo_test(|| {
        let v = read_todo_md_json("task_nonexistent_aaaaaaaaaaaaaaaa").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn update_todo_md_json_round_trip_consistent_with_list() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Update md", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");
        let content = "# Plan\n\nBody text\n";

        let updated = update_todo_md_json(master_id, content).expect("update");
        assert!(updated.get("_status").is_none());
        assert_eq!(updated["ok"], true);

        let read = read_todo_md_json(master_id).expect("read");
        assert_eq!(read["todo_md"], content);

        let listed = get_todo_tasks_json().expect("list");
        let task = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("listed");
        assert_eq!(task["todo_md"], content);
    });
}

#[test]
fn update_todo_md_json_missing_id_returns_400_class() {
    with_commands_todo_test(|| {
        let v = update_todo_md_json("", "# Plan").expect("invoke");
        assert_eq!(v["error"], "Missing id");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn update_todo_md_json_unknown_master_returns_404_class() {
    with_commands_todo_test(|| {
        let v = update_todo_md_json("task_nonexistent_aaaaaaaaaaaaaaaa", "# Plan")
            .expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn update_todo_md_json_io_failure_returns_500_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("IO fail cmd", Some(&["Sub"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        test_set_fail_batch_todo_md(true);
        let v = update_todo_md_json(master_id, "should not persist").expect("invoke");
        assert_eq!(v["_status"], 500);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn complete_todo_json_marks_sub_complete_without_rewriting_master() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Complete cmd", Some(&["A", "B"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        let completed = complete_todo_json(&master_id, Some(&sub_a)).expect("complete");
        assert!(completed.get("_status").is_none());
        let task = master_from_invoke(&completed);
        assert_eq!(task["status"], "incomplete");
        assert_eq!(task["sub_tasks"][0]["status"], "complete");
        assert_eq!(task["sub_tasks"][1]["status"], "incomplete");
    });
}

#[test]
fn complete_todo_json_without_sub_marks_master_complete() {
    with_commands_todo_test(|| {
        let created =
            create_todo_task_json("Complete master", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let completed = complete_todo_json(&master_id, None).expect("complete master");
        assert!(completed.get("_status").is_none());
        let task = master_from_invoke(&completed);
        assert_eq!(task["status"], "complete");
        assert_eq!(task["sub_tasks"][0]["status"], "incomplete");
        assert_master_task_shape(task);
    });
}

#[test]
fn complete_todo_json_already_complete_is_idempotent_success() {
    with_commands_todo_test(|| {
        let created =
            create_todo_task_json("Idempotent complete", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let first = complete_todo_json(&master_id, None).expect("first complete");
        assert!(first.get("_status").is_none());
        assert!(first.get("error").is_none());
        assert_eq!(master_from_invoke(&first)["status"], "complete");

        let again = complete_todo_json(&master_id, None).expect("second complete");
        assert!(again.get("_status").is_none());
        assert!(again.get("error").is_none());
        let task = master_from_invoke(&again);
        assert_eq!(task["status"], "complete");
        assert_master_task_shape(task);

        let listed = get_todo_tasks_json().expect("list");
        let found = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("listed");
        assert_eq!(found["status"], "complete");
    });
}

#[test]
fn complete_todo_json_abandoned_master_rejects_with_stable_code() {
    with_commands_todo_test(|| {
        let abandoned_id = "task_cmd_complete_abandoned";
        test_run_write_task_batch(
            abandoned_id,
            &IndexEntry {
                master_task_id: abandoned_id.to_string(),
                title: "Abandoned master".to_string(),
                status: MasterTaskStatus::Abandoned,
                created_at: "2026-07-08T00:00:00+00:00".to_string(),
                task_dir: format!("tasks/{abandoned_id}"),
                category_id: DEFAULT_CATEGORY_ID.to_string(),
            },
            &SubTasksFile { sub_tasks: vec![] },
            "",
        )
        .expect("seed abandoned");

        let rejected = complete_todo_json(abandoned_id, None).expect("invoke");
        assert_eq!(rejected["error"], "master_abandoned");
        assert_eq!(rejected["_status"], 409);
        assert!(rejected.get("task").is_none());

        let listed = get_todo_tasks_json().expect("list");
        let found = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == abandoned_id)
            .expect("listed");
        assert_eq!(found["status"], "abandoned");
        assert_master_task_shape(found);
    });
}

#[test]
fn complete_todo_json_abandoned_still_allows_non_status_edit() {
    with_commands_todo_test(|| {
        let abandoned_id = "task_cmd_abandoned_title_edit";
        test_run_write_task_batch(
            abandoned_id,
            &IndexEntry {
                master_task_id: abandoned_id.to_string(),
                title: "Abandoned title".to_string(),
                status: MasterTaskStatus::Abandoned,
                created_at: "2026-07-08T00:00:02+00:00".to_string(),
                task_dir: format!("tasks/{abandoned_id}"),
                category_id: DEFAULT_CATEGORY_ID.to_string(),
            },
            &SubTasksFile { sub_tasks: vec![] },
            "",
        )
        .expect("seed abandoned");

        let updated =
            update_todo_master_title_json(abandoned_id, "Still editable").expect("title edit");
        assert!(updated.get("_status").is_none());
        let task = master_from_invoke(&updated);
        assert_eq!(task["title"], "Still editable");
        assert_eq!(task["status"], "abandoned");
        assert_master_task_shape(task);
    });
}

#[test]
fn complete_todo_json_all_subs_done_leaves_master_incomplete() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("All subs done", Some(&["A", "B"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_ids: Vec<String> = master_from_invoke(&created)["sub_tasks"]
            .as_array()
            .expect("subs")
            .iter()
            .map(|s| s["sub_task_id"].as_str().expect("id").to_string())
            .collect();

        for sub_id in &sub_ids {
            let completed = complete_todo_json(&master_id, Some(sub_id)).expect("complete");
            assert!(completed.get("_status").is_none());
        }

        let listed = get_todo_tasks_json().expect("list");
        let task = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("task");
        assert_master_task_shape(task);
        assert_eq!(task["status"], "incomplete");
        for sub in task["sub_tasks"].as_array().expect("subs") {
            assert_eq!(sub["status"], "complete");
        }
    });
}

#[test]
fn create_todo_task_json_defaults_status_incomplete() {
    with_commands_todo_test(|| {
        let v = create_todo_task_json("Default status", None, "").expect("create");
        let task = master_from_invoke(&v);
        assert_eq!(task["status"], "incomplete");
        assert_master_task_shape(task);
    });
}

#[test]
fn todo_task_master_status_wire_includes_abandoned() {
    assert_eq!(
        super::TODO_TASK_MASTER_STATUS_WIRE,
        &["incomplete", "complete", "abandoned"]
    );
}

#[test]
fn get_todo_tasks_json_reads_back_complete_and_abandoned_status() {
    with_commands_todo_test(|| {
        let complete_id = "task_cmd_status_complete";
        let abandoned_id = "task_cmd_status_abandoned";
        test_run_write_task_batch(
            complete_id,
            &IndexEntry {
                master_task_id: complete_id.to_string(),
                title: "Stored complete".to_string(),
                status: MasterTaskStatus::Complete,
                created_at: "2026-07-08T00:00:00+00:00".to_string(),
                task_dir: format!("tasks/{complete_id}"),
                category_id: DEFAULT_CATEGORY_ID.to_string(),
            },
            &SubTasksFile { sub_tasks: vec![] },
            "",
        )
        .expect("seed complete");
        test_run_write_task_batch(
            abandoned_id,
            &IndexEntry {
                master_task_id: abandoned_id.to_string(),
                title: "Stored abandoned".to_string(),
                status: MasterTaskStatus::Abandoned,
                created_at: "2026-07-08T00:00:01+00:00".to_string(),
                task_dir: format!("tasks/{abandoned_id}"),
                category_id: DEFAULT_CATEGORY_ID.to_string(),
            },
            &SubTasksFile { sub_tasks: vec![] },
            "",
        )
        .expect("seed abandoned");

        let created = create_todo_task_json("Create incomplete", None, "").expect("create");
        let created_task = master_from_invoke(&created);
        assert_eq!(created_task["status"], "incomplete");
        assert_master_task_shape(created_task);

        let listed = get_todo_tasks_json().expect("list");
        let arr = listed.as_array().expect("array");
        let complete = arr
            .iter()
            .find(|t| t["master_task_id"] == complete_id)
            .expect("complete");
        let abandoned = arr
            .iter()
            .find(|t| t["master_task_id"] == abandoned_id)
            .expect("abandoned");
        assert_master_task_shape(complete);
        assert_master_task_shape(abandoned);
        assert_eq!(complete["status"], "complete");
        assert_eq!(abandoned["status"], "abandoned");
    });
}

#[test]
fn complete_todo_json_unknown_sub_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Complete 404", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = complete_todo_json(master_id, Some("task_missing_sub_01")).expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn complete_todo_json_terminal_sub_returns_409_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Terminal cmd", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        complete_todo_json(&master_id, Some(&sub_a)).expect("first complete");
        let again = complete_todo_json(&master_id, Some(&sub_a)).expect("invoke");
        assert_eq!(again["error"], "Sub task is in terminal status");
        assert_eq!(again["_status"], 409);
    });
}

#[test]
fn complete_todo_command_symbol_exists_for_handler_registration() {
    // Smoke: async command symbol exists for generate_handler! registration.
    let _ = complete_todo;
}

#[test]
fn abandon_todo_sub_json_unknown_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Abandon 404", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = abandon_todo_sub_json(master_id, "task_missing_sub_01").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn abandon_todo_sub_json_marks_sub_abandoned() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Abandon cmd", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        let abandoned = abandon_todo_sub_json(&master_id, &sub_a).expect("abandon");
        assert!(abandoned.get("_status").is_none());
        let task = master_from_invoke(&abandoned);
        assert_eq!(task["status"], "incomplete");
        assert_eq!(task["sub_tasks"][0]["status"], "abandoned");
    });
}

#[test]
fn abandon_todo_sub_json_terminal_returns_409_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Abandon terminal", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        abandon_todo_sub_json(&master_id, &sub_a).expect("first abandon");
        let again = abandon_todo_sub_json(&master_id, &sub_a).expect("invoke");
        assert_eq!(again["error"], "Sub task is in terminal status");
        assert_eq!(again["_status"], 409);
    });
}

#[test]
fn update_todo_master_title_json_success_strips_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Old cmd title", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let updated = update_todo_master_title_json(&master_id, "  Renamed  ").expect("update");
        assert!(updated.get("_status").is_none());
        let task = master_from_invoke(&updated);
        assert_master_task_shape(task);
        assert_eq!(task["title"], "Renamed");

        let listed = get_todo_tasks_json().expect("list");
        let found = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("listed");
        assert_eq!(found["title"], "Renamed");
    });
}

#[test]
fn update_todo_master_title_json_blank_returns_400_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Keep cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = update_todo_master_title_json(master_id, "  ").expect("invoke");
        assert_eq!(v["error"], "Missing title");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn update_todo_master_title_json_unknown_returns_404_class() {
    with_commands_todo_test(|| {
        let v = update_todo_master_title_json("task_nonexistent_aaaaaaaaaaaaaaaa", "New")
            .expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}


fn write_cmd_attach_source(name: &str, content: impl AsRef<[u8]>) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "todo_attach_cmd_{}_{}",
        std::process::id(),
        n
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path.canonicalize().unwrap_or(path)
}

fn add_attach_cmd(master_id: &str, name: &str, content: impl AsRef<[u8]>) -> serde_json::Value {
    let path = write_cmd_attach_source(name, content);
    add_todo_attachment_json(master_id, path.to_str().unwrap()).expect("add")
}

fn save_attach_cmd(
    master_id: &str,
    file_name: &str,
    source_name: &str,
    content: impl AsRef<[u8]>,
) -> serde_json::Value {
    let path = write_cmd_attach_source(source_name, content);
    save_todo_attachment_json(master_id, file_name, path.to_str().unwrap()).expect("save")
}

#[test]
fn todo_attachment_command_symbols_exist_for_handler_registration() {
    // Smoke: async command symbols exist for generate_handler! registration.
    let _ = stage_todo_attachment_source;
    let _ = add_todo_attachment;
    let _ = list_todo_attachments;
    let _ = read_todo_attachment;
    let _ = save_todo_attachment;
    let _ = delete_todo_attachment;
}

#[test]
fn add_todo_attachment_json_success_strips_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Attach cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let added = add_attach_cmd(&master_id, "notes.md", "# Notes\n");
        assert!(added.get("_status").is_none());
        assert_eq!(added["file_name"], "notes.md");
        assert_eq!(added["original_file_name"], "notes.md");
        assert!(added.get("added_at").and_then(|v| v.as_str()).is_some());
    });
}

#[test]
fn add_todo_attachment_json_rejects_non_md_with_400_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Attach nonmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = add_attach_cmd(master_id, "notes.txt", "nope");
        assert_eq!(v["error"], "Only .md attachments are supported");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn add_todo_attachment_json_unknown_master_returns_404_class() {
    with_commands_todo_test(|| {
        let v = add_attach_cmd("task_nonexistent_aaaaaaaaaaaaaaaa", "notes.md", "x");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn list_todo_attachments_json_returns_entries_without_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("List attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_attach_cmd(&master_id, "a.md", "one");
        add_attach_cmd(&master_id, "b.md", "two");

        let listed = list_todo_attachments_json(&master_id).expect("list");
        assert!(listed.get("_status").is_none());
        let attachments = listed["attachments"].as_array().expect("attachments");
        assert_eq!(attachments.len(), 2);
        assert_eq!(attachments[0]["file_name"], "a.md");
        assert_eq!(attachments[1]["file_name"], "b.md");
    });
}

#[test]
fn list_todo_attachments_json_unknown_returns_404_class() {
    with_commands_todo_test(|| {
        let v = list_todo_attachments_json("task_nonexistent_aaaaaaaaaaaaaaaa").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn read_todo_attachment_json_round_trip_content() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Read attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let content = "# Body\n";
        add_attach_cmd(&master_id, "notes.md", content);

        let read = read_todo_attachment_json(&master_id, "notes.md").expect("read");
        assert!(read.get("_status").is_none());
        assert_eq!(read["file_name"], "notes.md");
        assert_eq!(read["content"], content);
    });
}

#[test]
fn read_todo_attachment_json_missing_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Read missing", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = read_todo_attachment_json(master_id, "missing.md").expect("invoke");
        assert_eq!(v["error"], "Attachment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn save_todo_attachment_json_overwrites_content_without_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Save attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_attach_cmd(&master_id, "notes.md", "old");

        let saved = save_attach_cmd(&master_id, "notes.md", "notes.md", "new body");
        assert!(saved.get("_status").is_none());
        assert_eq!(saved["ok"], true);

        let read = read_todo_attachment_json(&master_id, "notes.md").expect("read");
        assert_eq!(read["content"], "new body");
    });
}

#[test]
fn save_todo_attachment_json_unknown_attachment_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Save missing", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let path = write_cmd_attach_source("nope.md", "x");
        let v = save_todo_attachment_json(master_id, "nope.md", path.to_str().unwrap()).expect("invoke");
        assert_eq!(v["error"], "Attachment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn delete_todo_attachment_json_removes_and_strips_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Del attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_attach_cmd(&master_id, "notes.md", "gone");

        let deleted = delete_todo_attachment_json(&master_id, "notes.md").expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed = list_todo_attachments_json(&master_id).expect("list");
        assert!(listed["attachments"].as_array().expect("arr").is_empty());

        let missing = read_todo_attachment_json(&master_id, "notes.md").expect("read");
        assert_eq!(missing["_status"], 404);
    });
}

#[test]
fn delete_todo_attachment_json_unknown_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Del missing", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = delete_todo_attachment_json(master_id, "nope.md").expect("invoke");
        assert_eq!(v["error"], "Attachment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn ac15_command_add_rejects_non_md_and_lists_empty() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("AC15 cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let listed = list_todo_attachments_json(master_id).expect("list");
        assert!(listed.get("_status").is_none());
        assert_eq!(listed["attachments"].as_array().expect("arr").len(), 0);

        let rejected = add_attach_cmd(master_id, "x.txt", "nope");
        assert_eq!(rejected["_status"], 400);
        assert_eq!(rejected["error"], "Only .md attachments are supported");
    });
}

#[test]
fn ac15_command_delete_dual_clear_via_list() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("AC15 del cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_attach_cmd(&master_id, "notes.md", "body");

        let deleted = delete_todo_attachment_json(&master_id, "notes.md").expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed = list_todo_attachments_json(&master_id).expect("list");
        assert!(listed["attachments"].as_array().expect("arr").is_empty());
    });
}

#[test]
fn todo_comment_command_symbols_exist_for_handler_registration() {
    // Smoke: async command symbols exist for generate_handler! registration.
    let _ = list_todo_comments;
    let _ = add_todo_comment;
    let _ = update_todo_comment;
    let _ = delete_todo_comment;
}

#[test]
fn add_todo_comment_json_success_strips_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Comment cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let added = add_todo_comment_json(&master_id, "process note").expect("add");
        assert!(added.get("_status").is_none());
        let id = added["id"].as_str().expect("id");
        assert!(id.starts_with("cmt_"), "id={id}");
        assert_eq!(added["body"], "process note");
        assert!(added.get("created_at").and_then(|v| v.as_str()).is_some());
    });
}

#[test]
fn add_todo_comment_json_rejects_empty_body_with_400_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Comment empty", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = add_todo_comment_json(master_id, "   ").expect("invoke");
        assert_eq!(v["error"], "Comment body must not be empty");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn add_todo_comment_json_unknown_master_returns_404_class() {
    with_commands_todo_test(|| {
        let v = add_todo_comment_json("task_nonexistent_aaaaaaaaaaaaaaaa", "x").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn list_todo_comments_json_returns_entries_without_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("List comments", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_todo_comment_json(&master_id, "first").expect("add a");
        add_todo_comment_json(&master_id, "second").expect("add b");

        let listed = list_todo_comments_json(&master_id).expect("list");
        assert!(listed.get("_status").is_none());
        let comments = listed["comments"].as_array().expect("comments");
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0]["body"], "first");
        assert_eq!(comments[1]["body"], "second");
    });
}

#[test]
fn list_todo_comments_json_unknown_returns_404_class() {
    with_commands_todo_test(|| {
        let v = list_todo_comments_json("task_nonexistent_aaaaaaaaaaaaaaaa").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn update_todo_comment_json_changes_body_without_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Update comment cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let added = add_todo_comment_json(&master_id, "old").expect("add");
        let comment_id = added["id"].as_str().expect("id").to_string();
        let created_at = added["created_at"].as_str().expect("created_at").to_string();

        let updated =
            update_todo_comment_json(&master_id, &comment_id, "new body").expect("update");
        assert!(updated.get("_status").is_none());
        assert_eq!(updated["id"], comment_id);
        assert_eq!(updated["body"], "new body");
        assert_eq!(updated["created_at"], created_at);

        let listed = list_todo_comments_json(&master_id).expect("list");
        let comments = listed["comments"].as_array().expect("comments");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0]["body"], "new body");
        assert_eq!(comments[0]["created_at"], created_at);
    });
}

#[test]
fn update_todo_comment_json_unknown_id_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Update missing", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = update_todo_comment_json(master_id, "cmt_missing00000001", "x").expect("invoke");
        assert_eq!(v["error"], "Comment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn delete_todo_comment_json_removes_and_strips_status() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Del comment cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let added = add_todo_comment_json(&master_id, "gone").expect("add");
        let comment_id = added["id"].as_str().expect("id").to_string();

        let deleted = delete_todo_comment_json(&master_id, &comment_id).expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed = list_todo_comments_json(&master_id).expect("list");
        assert!(listed["comments"].as_array().expect("arr").is_empty());
    });
}

#[test]
fn delete_todo_comment_json_unknown_returns_404_class() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Del missing comment", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = delete_todo_comment_json(master_id, "cmt_missing00000001").expect("invoke");
        assert_eq!(v["error"], "Comment not found");
        assert_eq!(v["_status"], 404);
    });
}

// --- t2: Host category Tauri command surface ---

#[test]
fn list_create_delete_todo_category_json_roundtrip() {
    with_commands_todo_test(|| {
        let listed = list_todo_categories_json().expect("list");
        assert!(listed.get("_status").is_none());
        let cats = listed["categories"].as_array().expect("categories");
        assert!(cats.iter().any(|c| c["id"] == DEFAULT_CATEGORY_ID));

        let created = create_todo_category_json("UI Cat").expect("create");
        assert!(created.get("_status").is_none());
        let cat_id = created["category_id"].as_str().expect("id").to_string();
        assert_eq!(created["category"]["name"], "UI Cat");

        let listed2 = list_todo_categories_json().expect("list2");
        assert!(listed2["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id));

        let deleted = delete_todo_category_json(&cat_id).expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed3 = list_todo_categories_json().expect("list3");
        assert!(!listed3["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id));
    });
}

#[test]
fn delete_todo_category_json_default_fails() {
    with_commands_todo_test(|| {
        let v = delete_todo_category_json(DEFAULT_CATEGORY_ID).expect("invoke");
        assert!(v.get("error").is_some());
        assert!(v["_status"].as_u64().unwrap_or(0) >= 400);
    });
}

#[test]
fn delete_todo_category_json_non_empty_fails() {
    with_commands_todo_test(|| {
        let created_cat = create_todo_category_json("Busy").expect("cat");
        let cat_id = created_cat["category_id"].as_str().expect("id").to_string();
        let created = create_todo_task_json("Member", None, "").expect("todo");
        let master_id = created["master_task_id"].as_str().expect("id");
        let set = set_todo_category_json(master_id, &cat_id).expect("set");
        assert!(set.get("_status").is_none() || set["_status"] == 200);
        assert_eq!(set["task"]["category_id"], cat_id);

        let v = delete_todo_category_json(&cat_id).expect("delete");
        assert!(v.get("error").is_some());
        assert!(v["_status"].as_u64().unwrap_or(0) >= 400);
    });
}

#[test]
fn set_todo_category_json_unknown_rejects() {
    with_commands_todo_test(|| {
        let created = create_todo_task_json("Set bad", None, "").expect("todo");
        let master_id = created["master_task_id"].as_str().expect("id");
        let v = set_todo_category_json(master_id, "cat_missing_zzz").expect("invoke");
        assert!(v.get("error").is_some());
        assert!(v["_status"].as_u64().unwrap_or(0) >= 400);
    });
}

#[test]
fn todo_category_commands_live_in_todo_task_not_write_rs() {
    let todo_cmds = include_str!("../../commands/todo_task.rs");
    assert!(
        todo_cmds.contains("list_todo_categories")
            && todo_cmds.contains("create_todo_category")
            && todo_cmds.contains("delete_todo_category"),
        "Host category commands must live in commands/todo_task.rs"
    );
    let write_rs = include_str!("../../commands/write.rs");
    assert!(
        !write_rs.contains("list_todo_categories")
            && !write_rs.contains("create_todo_category")
            && !write_rs.contains("delete_todo_category"),
        "todo category commands must not land in commands/write.rs"
    );
    assert!(
        !todo_cmds.contains("sediment_kb_list_todo")
            && !todo_cmds.contains("sediment_kb_create_todo")
            && !todo_cmds.contains("sediment_kb_delete_todo"),
        "must not use sediment_kb_* namespace for todo categories"
    );
}
