use super::*;

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
fn get_by_id_unknown_returns_404() {
    with_todo_task_sandbox(|_| {
        let v = get_by_id("task_does_not_exist");
        assert_eq!(v["_status"], 404);
        assert!(v.get("error").is_some());
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
fn create_response_omits_completed_at_on_new_subs() {
    with_todo_task_sandbox(|_| {
        let v = create_master_with_subs("New", Some(&["Sub"]));
        let subs = v["task"]["sub_tasks"].as_array().unwrap();
        assert!(subs[0].get("completed_at").is_none() || subs[0]["completed_at"].is_null());
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
