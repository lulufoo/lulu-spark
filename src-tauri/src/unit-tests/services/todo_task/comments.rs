use super::*;

#[test]
fn load_comments_missing_file_returns_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments missing file", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        assert!(!path.exists());

        let loaded = load_comments_file_unlocked(&path).expect("missing file is empty");
        assert!(loaded.comments.is_empty());
    });
}

#[test]
fn load_comments_empty_envelope_returns_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments empty envelope", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        fs::write(&path, r#"{ "comments": [] }"#).expect("write empty envelope");

        let loaded = load_comments_file_unlocked(&path).expect("empty envelope ok");
        assert!(loaded.comments.is_empty());
    });
}

#[test]
fn comments_file_round_trip_via_load_and_save() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments round-trip", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        let file = CommentsFile {
            comments: vec![CommentEntry {
                id: "cmt_abcdef012345".to_string(),
                body: "process note".to_string(),
                created_at: "2026-07-23T01:02:03+00:00".to_string(),
            }],
        };
        save_comments_file_unlocked(&path, &file).expect("save");
        assert!(path.is_file());

        let loaded = load_comments_file_unlocked(&path).expect("load");
        assert_eq!(loaded.comments.len(), 1);
        assert_eq!(loaded.comments[0].id, "cmt_abcdef012345");
        assert_eq!(loaded.comments[0].body, "process note");
        assert_eq!(loaded.comments[0].created_at, "2026-07-23T01:02:03+00:00");
    });
}

#[test]
fn mint_comment_id_and_created_at_follow_contract() {
    let id = mint_comment_id();
    assert!(id.starts_with("cmt_"), "id={id}");
    let hex = &id["cmt_".len()..];
    assert!(!hex.is_empty());
    assert!(
        hex.chars().all(|c| c.is_ascii_hexdigit()),
        "hex part must be hex: {hex}"
    );

    let created_at = now_comment_created_at();
    assert!(is_iso8601(&created_at), "created_at={created_at}");
}

#[test]
fn load_comments_bad_json_or_bare_array_is_explicit_error() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments bad json", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        fs::write(&path, "{not valid json").expect("write bad");
        let bad = load_comments_file_unlocked(&path);
        assert!(bad.is_err(), "bad JSON must error, not empty list");

        fs::write(
            &path,
            r#"[{"id":"cmt_x","body":"a","created_at":"2026-07-23T00:00:00+00:00"}]"#,
        )
        .expect("write bare array");
        let bare = load_comments_file_unlocked(&path);
        assert!(bare.is_err(), "bare array must error, not empty list");

        fs::write(&path, r#"{ "comments": "not-an-array" }"#).expect("write invalid envelope");
        let invalid = load_comments_file_unlocked(&path);
        assert!(invalid.is_err(), "invalid envelope must error");
    });
}

#[test]
fn validate_comment_body_rejects_empty_and_whitespace() {
    assert!(validate_comment_body("ok note").is_ok());
    assert!(validate_comment_body("").is_err());
    assert!(validate_comment_body("   ").is_err());
    assert!(validate_comment_body("\t\n").is_err());
}

#[test]
fn save_comments_file_does_not_enforce_count_cap() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Comments no count cap", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        let comments: Vec<CommentEntry> = (0..80)
            .map(|i| CommentEntry {
                id: format!("cmt_{i:012x}"),
                body: format!("body-{i}"),
                created_at: "2026-07-23T00:00:00+00:00".to_string(),
            })
            .collect();
        let file = CommentsFile { comments };
        save_comments_file_unlocked(&path, &file).expect("no count-cap on save");
        let loaded = load_comments_file_unlocked(&path).expect("load");
        assert_eq!(loaded.comments.len(), 80);
    });
}

#[test]
fn list_comments_missing_file_returns_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("List comments missing", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert!(!comments_json_abs_path(wb, master_id).exists());

        let v = list_comments(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["comments"].as_array().expect("comments array").len(), 0);
    });
}

#[test]
fn list_comments_orders_by_created_at_ascending() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("List comments order", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        let file = CommentsFile {
            comments: vec![
                CommentEntry {
                    id: "cmt_later000001".to_string(),
                    body: "second".to_string(),
                    created_at: "2026-07-23T02:00:00+00:00".to_string(),
                },
                CommentEntry {
                    id: "cmt_earlier0001".to_string(),
                    body: "first".to_string(),
                    created_at: "2026-07-23T01:00:00+00:00".to_string(),
                },
            ],
        };
        save_comments_file_unlocked(&path, &file).expect("seed");

        let v = list_comments(master_id);
        assert_eq!(v["_status"], 200);
        let items = v["comments"].as_array().expect("comments array");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "cmt_earlier0001");
        assert_eq!(items[0]["body"], "first");
        assert_eq!(items[1]["id"], "cmt_later000001");
        assert_eq!(items[1]["body"], "second");
    });
}

#[test]
fn add_comment_writes_new_id_and_created_at_then_list_shows_it() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Add comment", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        let added = add_comment(master_id, "process note");
        assert_eq!(added["_status"], 201);
        let id = added["id"].as_str().expect("id");
        assert!(id.starts_with("cmt_"), "id={id}");
        let created_at = added["created_at"].as_str().expect("created_at");
        assert!(is_iso8601(created_at), "created_at={created_at}");
        assert_eq!(added["body"], "process note");

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], id);
        assert_eq!(items[0]["body"], "process note");
        assert_eq!(items[0]["created_at"], created_at);
    });
}

#[test]
fn update_comment_changes_only_body_preserving_id_created_at_and_order() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Update comment", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        let file = CommentsFile {
            comments: vec![
                CommentEntry {
                    id: "cmt_a00000000001".to_string(),
                    body: "old-a".to_string(),
                    created_at: "2026-07-23T01:00:00+00:00".to_string(),
                },
                CommentEntry {
                    id: "cmt_b00000000002".to_string(),
                    body: "old-b".to_string(),
                    created_at: "2026-07-23T02:00:00+00:00".to_string(),
                },
            ],
        };
        save_comments_file_unlocked(&path, &file).expect("seed");

        let updated = update_comment(master_id, "cmt_a00000000001", "new-a");
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["id"], "cmt_a00000000001");
        assert_eq!(updated["body"], "new-a");
        assert_eq!(updated["created_at"], "2026-07-23T01:00:00+00:00");

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "cmt_a00000000001");
        assert_eq!(items[0]["body"], "new-a");
        assert_eq!(items[0]["created_at"], "2026-07-23T01:00:00+00:00");
        assert_eq!(items[1]["id"], "cmt_b00000000002");
        assert_eq!(items[1]["body"], "old-b");
    });
}

#[test]
fn delete_comment_hard_removes_id_from_list() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Delete comment", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let a = add_comment(master_id, "keep");
        let b = add_comment(master_id, "drop");
        assert_eq!(a["_status"], 201);
        assert_eq!(b["_status"], 201);
        let drop_id = b["id"].as_str().unwrap().to_string();

        let deleted = delete_comment(master_id, &drop_id);
        assert_eq!(deleted["_status"], 200);
        assert_eq!(deleted["ok"], true);

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["id"], a["id"]);
        assert!(items.iter().all(|c| c["id"] != drop_id));
    });
}

#[test]
fn add_comment_repeatedly_is_not_rejected_for_count() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Add comment no cap", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        for i in 0..12 {
            let v = add_comment(master_id, &format!("note-{i}"));
            assert_eq!(v["_status"], 201, "add #{i} must not be rejected for count");
        }

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        assert_eq!(listed["comments"].as_array().expect("comments").len(), 12);
    });
}

#[test]
fn delete_master_cascades_comments_json_via_remove_dir_all() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Delete master comments cascade", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let added = add_comment(master_id, "will cascade away");
        assert_eq!(added["_status"], 201);

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        let comments_path = comments_json_abs_path(wb, master_id);
        assert!(task_dir.is_dir());
        assert!(comments_path.is_file());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert_eq!(v["ok"], true);
        assert!(!task_dir.exists());
        assert!(!comments_path.exists());
    });
}

#[test]
fn add_and_update_comment_reject_empty_or_whitespace_body() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Empty body reject", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        for bad in ["", "   ", "\t\n"] {
            let add = add_comment(master_id, bad);
            assert_ne!(add["_status"], 201, "empty body add must fail");
            assert!(add.get("error").is_some(), "add must be explicit error");
        }
        assert!(!comments_json_abs_path(wb, master_id).exists());

        let ok = add_comment(master_id, "seed");
        assert_eq!(ok["_status"], 201);
        let id = ok["id"].as_str().unwrap();

        for bad in ["", "   ", "\t\n"] {
            let upd = update_comment(master_id, id, bad);
            assert_ne!(upd["_status"], 200, "empty body update must fail");
            assert!(upd.get("error").is_some(), "update must be explicit error");
        }

        let listed = list_comments(master_id);
        assert_eq!(listed["comments"][0]["body"], "seed");
    });
}

#[test]
fn update_and_delete_comment_unknown_id_is_explicit_error() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("Unknown comment id", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let ok = add_comment(master_id, "exists");
        assert_eq!(ok["_status"], 201);

        let upd = update_comment(master_id, "cmt_doesnotexist", "x");
        assert_ne!(upd["_status"], 200);
        assert!(upd.get("error").is_some());

        let del = delete_comment(master_id, "cmt_doesnotexist");
        assert_ne!(del["_status"], 200);
        assert!(del.get("error").is_some());

        let listed = list_comments(master_id);
        assert_eq!(listed["comments"].as_array().unwrap().len(), 1);
    });
}

#[test]
fn bad_comments_json_errors_on_list_and_writes_without_overwrite() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Bad comments json", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);
        let bad = "{not valid json";
        fs::write(&path, bad).expect("seed bad");

        let listed = list_comments(master_id);
        assert_ne!(listed["_status"], 200);
        assert!(listed.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);

        let add = add_comment(master_id, "should fail");
        assert_ne!(add["_status"], 201);
        assert!(add.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);

        let upd = update_comment(master_id, "cmt_x", "nope");
        assert_ne!(upd["_status"], 200);
        assert!(upd.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);

        let del = delete_comment(master_id, "cmt_x");
        assert_ne!(del["_status"], 200);
        assert!(del.get("error").is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);
    });
}

// --- T5 AC regression locks (tech-doc T5 / AC1,AC4,AC5,AC9 / 不变量#2,#4,#5,#6) ---

#[test]
fn t5_add_list_orders_by_created_at_update_preserves_order_hard_delete_removes() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 CRUD order hard-delete", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        // Seed out-of-order created_at so list must sort ascending (AC2).
        let path = comments_json_abs_path(wb, master_id);
        let file = CommentsFile {
            comments: vec![
                CommentEntry {
                    id: "cmt_later000001".to_string(),
                    body: "second".to_string(),
                    created_at: "2026-07-23T02:00:00+00:00".to_string(),
                },
                CommentEntry {
                    id: "cmt_earlier0001".to_string(),
                    body: "first".to_string(),
                    created_at: "2026-07-23T01:00:00+00:00".to_string(),
                },
            ],
        };
        save_comments_file_unlocked(&path, &file).expect("seed");

        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        let items = listed["comments"].as_array().expect("comments");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["id"], "cmt_earlier0001");
        assert_eq!(items[1]["id"], "cmt_later000001");

        // update preserves id/created_at/order (AC3).
        let updated = update_comment(master_id, "cmt_earlier0001", "first-edited");
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["id"], "cmt_earlier0001");
        assert_eq!(updated["created_at"], "2026-07-23T01:00:00+00:00");
        let after_update = list_comments(master_id);
        let after_items = after_update["comments"].as_array().unwrap();
        assert_eq!(after_items[0]["id"], "cmt_earlier0001");
        assert_eq!(after_items[0]["body"], "first-edited");
        assert_eq!(after_items[1]["id"], "cmt_later000001");

        // Hard-delete by id — intentional Spec↔Design deviation (P4 / 不变量#5):
        // ordinary single-entry hard delete; no soft-delete / version-audit path (AC4).
        let deleted = delete_comment(master_id, "cmt_later000001");
        assert_eq!(deleted["_status"], 200);
        assert_eq!(deleted["ok"], true);
        assert!(deleted.get("soft_deleted").is_none());
        assert!(deleted.get("deleted_at").is_none());
        assert!(deleted.get("audit").is_none());

        let after_delete = list_comments(master_id);
        let remain = after_delete["comments"].as_array().unwrap();
        assert_eq!(remain.len(), 1);
        assert_eq!(remain[0]["id"], "cmt_earlier0001");
        assert!(remain.iter().all(|c| c["id"] != "cmt_later000001"));

        // Disk must not keep a soft-delete tombstone for the removed id.
        let on_disk: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let disk_ids: Vec<&str> = on_disk["comments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert!(!disk_ids.contains(&"cmt_later000001"));
        for entry in on_disk["comments"].as_array().unwrap() {
            let obj = entry.as_object().unwrap();
            assert!(!obj.contains_key("deleted"));
            assert!(!obj.contains_key("deleted_at"));
            assert!(!obj.contains_key("is_deleted"));
            assert!(!obj.contains_key("version"));
            assert!(!obj.contains_key("updated_at"));
            assert_eq!(obj.len(), 3, "comment fields only id/body/created_at");
        }
    });
}

#[test]
fn t5_delete_master_cascades_comments_json_with_task_dir() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 master cascade comments", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        assert_eq!(add_comment(master_id, "cascade-me")["_status"], 201);

        let task_dir = wb.join("todo_tasks").join("tasks").join(master_id);
        let comments_path = comments_json_abs_path(wb, master_id);
        assert!(task_dir.is_dir());
        assert!(comments_path.is_file());

        let v = delete_master(master_id);
        assert_eq!(v["_status"], 200);
        assert!(!task_dir.exists());
        assert!(!comments_path.exists(), "comments.json must vanish with task dir (AC5)");
    });
}

#[test]
fn t5_repeated_nonempty_add_is_not_rejected_for_count() {
    with_todo_task_sandbox(|_| {
        let created = create_master_with_subs("T5 no count cap", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        // AC9 / 不变量#6: add must not refuse based on existing count.
        for i in 0..20 {
            let v = add_comment(master_id, &format!("t5-note-{i}"));
            assert_eq!(
                v["_status"], 201,
                "add #{i} must succeed; no count-cap rejection"
            );
        }
        let listed = list_comments(master_id);
        assert_eq!(listed["_status"], 200);
        assert_eq!(listed["comments"].as_array().unwrap().len(), 20);
    });
}

#[test]
fn t5_list_missing_file_and_empty_envelope_are_empty_not_error() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 empty list semantics", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        assert!(!path.exists());
        let missing = list_comments(master_id);
        assert_eq!(missing["_status"], 200);
        assert_eq!(missing["comments"].as_array().unwrap().len(), 0);

        fs::write(&path, r#"{"comments":[]}"#).unwrap();
        let empty = list_comments(master_id);
        assert_eq!(empty["_status"], 200);
        assert_eq!(empty["comments"].as_array().unwrap().len(), 0);
    });
}

#[test]
fn t5_empty_or_whitespace_body_rejected_on_add_and_update() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 empty body", None);
        let master_id = created["master_task_id"].as_str().unwrap();

        for bad in ["", "   ", "\t\n", " \t "] {
            let add = add_comment(master_id, bad);
            assert_eq!(add["_status"], 400, "empty/whitespace add must 400");
            assert!(add.get("error").is_some());
        }
        assert!(!comments_json_abs_path(wb, master_id).exists());

        let ok = add_comment(master_id, "seed-body");
        assert_eq!(ok["_status"], 201);
        let id = ok["id"].as_str().unwrap();
        for bad in ["", "   ", "\t\n"] {
            let upd = update_comment(master_id, id, bad);
            assert_eq!(upd["_status"], 400, "empty/whitespace update must 400");
            assert!(upd.get("error").is_some());
        }
        assert_eq!(list_comments(master_id)["comments"][0]["body"], "seed-body");
    });
}

#[test]
fn t5_bad_json_and_invalid_envelope_are_explicit_errors_not_empty_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 bad json envelope", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        let cases = [
            "{not valid json",
            r#"[{"id":"cmt_x","body":"a","created_at":"2026-07-23T00:00:00+00:00"}]"#,
            r#"{"comments":"not-an-array"}"#,
            "", // empty file = corrupt, not empty-list success (AC1)
        ];
        for bad in cases {
            fs::write(&path, bad).unwrap();
            let listed = list_comments(master_id);
            assert_ne!(
                listed["_status"], 200,
                "corrupt comments.json must not succeed; payload={bad:?}"
            );
            assert!(listed.get("error").is_some());
            // Must not equal empty-list success semantics (缺文件/合法空 envelope).
            let disguised_empty = listed["_status"] == 200
                && listed
                    .get("comments")
                    .and_then(|c| c.as_array())
                    .map(|a| a.is_empty())
                    .unwrap_or(false);
            assert!(
                !disguised_empty,
                "bad JSON/envelope must not look like empty list"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), bad);
        }
    });
}

#[test]
fn t5_soft_delete_or_audit_fields_are_explicit_errors_not_accepted_schema() {
    // Spec「删除特判」偏差为有意：普通单条硬删除；无 soft-delete / 版本审计路径（不变量#5 / T5）。
    // Soft-delete-shaped or audit fields must be rejected as invalid envelope — not silently loaded.
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("T5 reject soft-delete fields", None);
        let master_id = created["master_task_id"].as_str().unwrap();
        let path = comments_json_abs_path(wb, master_id);

        fs::write(
            &path,
            r#"{
              "comments": [{
                "id": "cmt_soft0000001",
                "body": "should not load",
                "created_at": "2026-07-23T00:00:00+00:00",
                "deleted": true,
                "deleted_at": "2026-07-23T01:00:00+00:00"
              }]
            }"#,
        )
        .unwrap();

        let listed = list_comments(master_id);
        assert_ne!(
            listed["_status"], 200,
            "soft-delete fields must be explicit error, not accepted schema"
        );
        assert!(listed.get("error").is_some());
        let disguised_empty = listed["_status"] == 200
            && listed
                .get("comments")
                .and_then(|c| c.as_array())
                .map(|a| a.is_empty())
                .unwrap_or(false);
        assert!(!disguised_empty);

        // version-audit shaped field likewise rejected
        fs::write(
            &path,
            r#"{
              "comments": [{
                "id": "cmt_audit000001",
                "body": "should not load",
                "created_at": "2026-07-23T00:00:00+00:00",
                "version": 1
              }]
            }"#,
        )
        .unwrap();
        let listed_audit = list_comments(master_id);
        assert_ne!(listed_audit["_status"], 200);
        assert!(listed_audit.get("error").is_some());
    });
}
