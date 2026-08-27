use super::*;

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
