use super::*;

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
