use super::*;

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
