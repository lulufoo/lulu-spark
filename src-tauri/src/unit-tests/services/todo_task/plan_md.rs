use super::*;

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
