use std::fs;

use serde_json::json;

use crate::commands::plan_task::{
    abandon_plan_sub_json, add_plan_attachment, add_plan_attachment_json, add_plan_sub_json,
    complete_plan_sub_json, create_plan_task_json, delete_plan_attachment,
    delete_plan_attachment_json, delete_plan_sub_json, delete_plan_task_json,
    get_plan_tasks_json, list_plan_attachments, list_plan_attachments_json,
    read_plan_attachment, read_plan_attachment_json, read_plan_md_json,
    save_plan_attachment, save_plan_attachment_json, update_plan_master_title_json,
    update_plan_md_json,
};
use crate::services::plan_task::{
    create_master_with_subs, list_all, test_reset_all_injection_flags, test_set_fail_batch_plan_md,
};
use crate::test_support::TestSandbox;

fn master_from_invoke(v: &serde_json::Value) -> &serde_json::Value {
    v.get("task").expect("task field")
}

fn with_commands_plan_test<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    test_reset_all_injection_flags();
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
        let v = create_plan_task_json("Empty cmd", None, "").expect("create");
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
        let v = create_plan_task_json("Multi cmd", Some(&["A", "B"]), "").expect("create");
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
        let v = create_plan_task_json("", None, "").expect("invoke");
        assert_eq!(v["error"], "Missing title");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn delete_plan_task_json_removes_master_from_list() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Delete me", None, "").expect("create");
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
        let created = create_plan_task_json("Add sub", Some(&["A"]), "").expect("create");
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
        let created = create_plan_task_json("Del sub", Some(&["A", "B"]), "").expect("create");
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
        let created = create_plan_task_json("Last sub", Some(&["Only"]), "")
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

#[test]
fn read_plan_md_json_returns_plan_md_without_status() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Plan md cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let read = read_plan_md_json(master_id).expect("read");
        assert!(read.get("_status").is_none());
        assert_eq!(read["plan_md"], "");
    });
}

#[test]
fn read_plan_md_json_missing_id_returns_400_class() {
    with_commands_plan_test(|| {
        let v = read_plan_md_json("").expect("invoke");
        assert_eq!(v["error"], "Missing id");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn read_plan_md_json_unknown_master_returns_404_class() {
    with_commands_plan_test(|| {
        let v = read_plan_md_json("task_nonexistent_aaaaaaaaaaaaaaaa").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn update_plan_md_json_round_trip_consistent_with_list() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Update md", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");
        let content = "# Plan\n\nBody text\n";

        let updated = update_plan_md_json(master_id, content).expect("update");
        assert!(updated.get("_status").is_none());
        assert_eq!(updated["ok"], true);

        let read = read_plan_md_json(master_id).expect("read");
        assert_eq!(read["plan_md"], content);

        let listed = get_plan_tasks_json().expect("list");
        let task = listed
            .as_array()
            .expect("array")
            .iter()
            .find(|t| t["master_task_id"] == master_id)
            .expect("listed");
        assert_eq!(task["plan_md"], content);
    });
}

#[test]
fn update_plan_md_json_missing_id_returns_400_class() {
    with_commands_plan_test(|| {
        let v = update_plan_md_json("", "# Plan").expect("invoke");
        assert_eq!(v["error"], "Missing id");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn update_plan_md_json_unknown_master_returns_404_class() {
    with_commands_plan_test(|| {
        let v = update_plan_md_json("task_nonexistent_aaaaaaaaaaaaaaaa", "# Plan")
            .expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn update_plan_md_json_io_failure_returns_500_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("IO fail cmd", Some(&["Sub"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        test_set_fail_batch_plan_md(true);
        let v = update_plan_md_json(master_id, "should not persist").expect("invoke");
        assert_eq!(v["_status"], 500);
        assert!(v.get("error").is_some());
    });
}

#[test]
fn complete_plan_sub_json_marks_sub_complete_and_recomputes_master() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Complete cmd", Some(&["A", "B"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        let completed = complete_plan_sub_json(&master_id, &sub_a).expect("complete");
        assert!(completed.get("_status").is_none());
        let task = master_from_invoke(&completed);
        assert_eq!(task["status"], "incomplete");
        assert_eq!(task["sub_tasks"][0]["status"], "complete");
        assert_eq!(task["sub_tasks"][1]["status"], "incomplete");
    });
}

#[test]
fn complete_plan_sub_json_unknown_returns_404_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Complete 404", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = complete_plan_sub_json(master_id, "task_missing_sub_01").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn complete_plan_sub_json_terminal_returns_409_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Terminal cmd", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        complete_plan_sub_json(&master_id, &sub_a).expect("first complete");
        let again = complete_plan_sub_json(&master_id, &sub_a).expect("invoke");
        assert_eq!(again["error"], "Sub task is in terminal status");
        assert_eq!(again["_status"], 409);
    });
}

#[test]
fn abandon_plan_sub_json_unknown_returns_404_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Abandon 404", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = abandon_plan_sub_json(master_id, "task_missing_sub_01").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn abandon_plan_sub_json_marks_sub_abandoned() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Abandon cmd", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        let abandoned = abandon_plan_sub_json(&master_id, &sub_a).expect("abandon");
        assert!(abandoned.get("_status").is_none());
        let task = master_from_invoke(&abandoned);
        assert_eq!(task["status"], "incomplete");
        assert_eq!(task["sub_tasks"][0]["status"], "abandoned");
    });
}

#[test]
fn abandon_plan_sub_json_terminal_returns_409_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Abandon terminal", Some(&["A"]), "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let sub_a = master_from_invoke(&created)["sub_tasks"][0]["sub_task_id"]
            .as_str()
            .expect("sub_a")
            .to_string();

        abandon_plan_sub_json(&master_id, &sub_a).expect("first abandon");
        let again = abandon_plan_sub_json(&master_id, &sub_a).expect("invoke");
        assert_eq!(again["error"], "Sub task is in terminal status");
        assert_eq!(again["_status"], 409);
    });
}

#[test]
fn update_plan_master_title_json_success_strips_status() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Old cmd title", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let updated = update_plan_master_title_json(&master_id, "  Renamed  ").expect("update");
        assert!(updated.get("_status").is_none());
        let task = master_from_invoke(&updated);
        assert_master_task_shape(task);
        assert_eq!(task["title"], "Renamed");

        let listed = get_plan_tasks_json().expect("list");
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
fn update_plan_master_title_json_blank_returns_400_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Keep cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = update_plan_master_title_json(master_id, "  ").expect("invoke");
        assert_eq!(v["error"], "Missing title");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn update_plan_master_title_json_unknown_returns_404_class() {
    with_commands_plan_test(|| {
        let v = update_plan_master_title_json("task_nonexistent_aaaaaaaaaaaaaaaa", "New")
            .expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn plan_attachment_command_symbols_exist_for_handler_registration() {
    // Smoke: async command symbols exist for generate_handler! registration.
    let _ = add_plan_attachment;
    let _ = list_plan_attachments;
    let _ = read_plan_attachment;
    let _ = save_plan_attachment;
    let _ = delete_plan_attachment;
}

#[test]
fn add_plan_attachment_json_success_strips_status() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Attach cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();

        let added = add_plan_attachment_json(&master_id, "notes.md", "# Notes\n").expect("add");
        assert!(added.get("_status").is_none());
        assert_eq!(added["file_name"], "notes.md");
        assert_eq!(added["original_file_name"], "notes.md");
        assert!(added.get("added_at").and_then(|v| v.as_str()).is_some());
    });
}

#[test]
fn add_plan_attachment_json_rejects_non_md_with_400_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Attach nonmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = add_plan_attachment_json(master_id, "notes.txt", "nope").expect("invoke");
        assert_eq!(v["error"], "Only .md attachments are supported");
        assert_eq!(v["_status"], 400);
    });
}

#[test]
fn add_plan_attachment_json_unknown_master_returns_404_class() {
    with_commands_plan_test(|| {
        let v = add_plan_attachment_json(
            "task_nonexistent_aaaaaaaaaaaaaaaa",
            "notes.md",
            "x",
        )
        .expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn list_plan_attachments_json_returns_entries_without_status() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("List attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_plan_attachment_json(&master_id, "a.md", "one").expect("add a");
        add_plan_attachment_json(&master_id, "b.md", "two").expect("add b");

        let listed = list_plan_attachments_json(&master_id).expect("list");
        assert!(listed.get("_status").is_none());
        let attachments = listed["attachments"].as_array().expect("attachments");
        assert_eq!(attachments.len(), 2);
        assert_eq!(attachments[0]["file_name"], "a.md");
        assert_eq!(attachments[1]["file_name"], "b.md");
    });
}

#[test]
fn list_plan_attachments_json_unknown_returns_404_class() {
    with_commands_plan_test(|| {
        let v = list_plan_attachments_json("task_nonexistent_aaaaaaaaaaaaaaaa").expect("invoke");
        assert_eq!(v["error"], "Task not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn read_plan_attachment_json_round_trip_content() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Read attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        let content = "# Body\n";
        add_plan_attachment_json(&master_id, "notes.md", content).expect("add");

        let read = read_plan_attachment_json(&master_id, "notes.md").expect("read");
        assert!(read.get("_status").is_none());
        assert_eq!(read["file_name"], "notes.md");
        assert_eq!(read["content"], content);
    });
}

#[test]
fn read_plan_attachment_json_missing_returns_404_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Read missing", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = read_plan_attachment_json(master_id, "missing.md").expect("invoke");
        assert_eq!(v["error"], "Attachment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn save_plan_attachment_json_overwrites_content_without_status() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Save attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_plan_attachment_json(&master_id, "notes.md", "old").expect("add");

        let saved = save_plan_attachment_json(&master_id, "notes.md", "new body").expect("save");
        assert!(saved.get("_status").is_none());
        assert_eq!(saved["ok"], true);

        let read = read_plan_attachment_json(&master_id, "notes.md").expect("read");
        assert_eq!(read["content"], "new body");
    });
}

#[test]
fn save_plan_attachment_json_unknown_attachment_returns_404_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Save missing", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = save_plan_attachment_json(master_id, "nope.md", "x").expect("invoke");
        assert_eq!(v["error"], "Attachment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn delete_plan_attachment_json_removes_and_strips_status() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Del attach", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_plan_attachment_json(&master_id, "notes.md", "gone").expect("add");

        let deleted = delete_plan_attachment_json(&master_id, "notes.md").expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed = list_plan_attachments_json(&master_id).expect("list");
        assert!(listed["attachments"].as_array().expect("arr").is_empty());

        let missing = read_plan_attachment_json(&master_id, "notes.md").expect("read");
        assert_eq!(missing["_status"], 404);
    });
}

#[test]
fn delete_plan_attachment_json_unknown_returns_404_class() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("Del missing", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let v = delete_plan_attachment_json(master_id, "nope.md").expect("invoke");
        assert_eq!(v["error"], "Attachment not found");
        assert_eq!(v["_status"], 404);
    });
}

#[test]
fn ac15_command_add_rejects_non_md_and_lists_empty() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("AC15 cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id");

        let listed = list_plan_attachments_json(master_id).expect("list");
        assert!(listed.get("_status").is_none());
        assert_eq!(listed["attachments"].as_array().expect("arr").len(), 0);

        let rejected = add_plan_attachment_json(master_id, "x.txt", "nope").expect("invoke");
        assert_eq!(rejected["_status"], 400);
        assert_eq!(rejected["error"], "Only .md attachments are supported");
    });
}

#[test]
fn ac15_command_delete_dual_clear_via_list() {
    with_commands_plan_test(|| {
        let created = create_plan_task_json("AC15 del cmd", None, "").expect("create");
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        add_plan_attachment_json(&master_id, "notes.md", "body").expect("add");

        let deleted = delete_plan_attachment_json(&master_id, "notes.md").expect("delete");
        assert!(deleted.get("_status").is_none());
        assert_eq!(deleted["ok"], true);

        let listed = list_plan_attachments_json(&master_id).expect("list");
        assert!(listed["attachments"].as_array().expect("arr").is_empty());
    });
}
