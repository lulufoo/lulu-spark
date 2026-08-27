use super::*;

#[test]
fn todo_tasks_categories_path_is_ssot_under_todo_tasks() {
    with_todo_task_sandbox(|wb| {
        let path = paths::todo_tasks_categories_path().expect("path");
        assert_eq!(path, wb.join("todo_tasks").join("categories.json"));
        let sediment = paths::sediment_kb_categories_path().expect("sediment");
        assert_ne!(path, sediment);
    });
}

#[test]
fn load_categories_injects_default_when_registry_missing() {
    with_todo_task_sandbox(|wb| {
        let cats_path = wb.join("todo_tasks").join("categories.json");
        assert!(!cats_path.exists());
        let cats = load_categories().expect("load");
        assert!(
            cats.categories.iter().any(|c| {
                c.id == DEFAULT_CATEGORY_ID
                    && c.name == DEFAULT_CATEGORY_NAME
                    && c.is_default
            }),
            "missing registry must inject built-in 待分类"
        );
    });
}

#[test]
fn ensure_default_category_persists_registry_via_paths() {
    with_todo_task_sandbox(|wb| {
        let cats = ensure_default_category().expect("ensure");
        assert!(cats
            .categories
            .iter()
            .any(|c| c.id == DEFAULT_CATEGORY_ID && c.is_default));
        let path = paths::todo_tasks_categories_path().expect("path");
        assert_eq!(path, wb.join("todo_tasks").join("categories.json"));
        assert!(path.is_file(), "ensure may create default registry file");
        let text = fs::read_to_string(&path).expect("read");
        assert!(text.contains(DEFAULT_CATEGORY_ID));
        assert!(text.contains(DEFAULT_CATEGORY_NAME));
    });
}

#[test]
fn missing_categories_registry_does_not_block_todo_list() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("No cats yet", None);
        assert_eq!(created["_status"], 201);
        let cats_path = wb.join("todo_tasks").join("categories.json");
        if cats_path.is_file() {
            fs::remove_file(&cats_path).expect("remove cats");
        }
        let listed = list_all();
        let arr = listed.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["title"], "No cats yet");
    });
}

#[test]
fn todo_missing_category_id_reads_as_default_without_rewriting_disk() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_subs("Legacy todo", None);
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

        let before = fs::read_to_string(&index_path).unwrap();
        assert!(!before.contains("\"category_id\""));

        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], DEFAULT_CATEGORY_ID);

        let after = fs::read_to_string(&index_path).unwrap();
        assert_eq!(before, after, "compat read must not rewrite disk");
    });
}

#[test]
fn todo_orphan_category_id_reads_as_default_without_rewriting_disk() {
    with_todo_task_sandbox(|wb| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_subs("Orphan cat", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        let index_path = wb.join("todo_tasks").join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["tasks"][master_id]["category_id"] = serde_json::json!("cat_missing_zzz");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let before = fs::read_to_string(&index_path).unwrap();
        assert!(before.contains("cat_missing_zzz"));

        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], DEFAULT_CATEGORY_ID);

        let after = fs::read_to_string(&index_path).unwrap();
        assert_eq!(before, after, "orphan compat read must not rewrite disk");
        assert!(after.contains("cat_missing_zzz"));
    });
}

#[test]
fn todo_valid_category_id_roundtrips_on_read() {
    with_todo_task_sandbox(|wb| {
        ensure_default_category().expect("ensure");
        let cats_path = paths::todo_tasks_categories_path().expect("path");
        let mut cats = load_categories().expect("load");
        cats.categories
            .push(crate::services::todo_task::types::Category {
                id: "cat_work".to_string(),
                name: "Work".to_string(),
                is_default: false,
            });
        fs::write(&cats_path, serde_json::to_string_pretty(&cats).unwrap()).unwrap();

        let created = create_master_with_subs("Work todo", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        let index_path = wb.join("todo_tasks").join("index.json");
        let mut index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        index["tasks"][master_id]["category_id"] = serde_json::json!("cat_work");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], "cat_work");
        let listed = list_all();
        assert_eq!(listed.as_array().unwrap()[0]["category_id"], "cat_work");
    });
}

// --- t2: Host category rules, migration, create/set category ---

#[test]
fn create_without_category_writes_default_uncategorized_id() {
    with_todo_task_sandbox(|wb| {
        let created = create_master_with_category("Default cat", None, "", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id");
        assert_eq!(created["task"]["category_id"], DEFAULT_CATEGORY_ID);

        let index_path = wb.join("todo_tasks").join("index.json");
        let index: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
        assert_eq!(
            index["tasks"][master_id]["category_id"],
            DEFAULT_CATEGORY_ID,
            "create omit must persist default 待分类 id"
        );
    });
}

#[test]
fn create_with_valid_category_id_assigns_that_category() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let created_cat = create_todo_category("Work");
        assert_eq!(created_cat["_status"], 201);
        let cat_id = created_cat["category_id"].as_str().expect("cat id").to_string();

        let created = create_master_with_category("In work", None, "", Some(&cat_id));
        assert_eq!(created["_status"], 201);
        assert_eq!(created["task"]["category_id"], cat_id);
        let master_id = created["master_task_id"].as_str().expect("id");
        let got = get_by_id(master_id);
        assert_eq!(got["category_id"], cat_id);
    });
}

#[test]
fn create_with_unknown_category_id_rejects_without_fallback() {
    with_todo_task_sandbox(|wb| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_category("Bad cat", None, "", Some("cat_missing_zzz"));
        assert!(
            created["_status"].as_u64().unwrap_or(0) >= 400,
            "unknown category_id must reject"
        );
        assert!(created.get("error").is_some());

        let index_path = wb.join("todo_tasks").join("index.json");
        if index_path.is_file() {
            let index: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
            let tasks = index["tasks"].as_object().expect("tasks");
            assert!(
                tasks.is_empty(),
                "rejected create must not persist a todo"
            );
        }
    });
}

#[test]
fn set_master_category_valid_id_updates_membership() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let cat = create_todo_category("Later");
        let cat_id = cat["category_id"].as_str().expect("id").to_string();
        let created = create_master_with_category("Move me", None, "", None);
        let master_id = created["master_task_id"].as_str().expect("id");

        let updated = set_master_category(master_id, &cat_id);
        assert_eq!(updated["_status"], 200);
        assert_eq!(updated["task"]["category_id"], cat_id);
        assert_eq!(get_by_id(master_id)["category_id"], cat_id);
    });
}

#[test]
fn set_master_category_unknown_id_rejects_without_fallback() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_category("Keep default", None, "", None);
        let master_id = created["master_task_id"].as_str().expect("id");

        let updated = set_master_category(master_id, "cat_does_not_exist");
        assert!(updated["_status"].as_u64().unwrap_or(0) >= 400);
        assert!(updated.get("error").is_some());
        assert_eq!(
            get_by_id(master_id)["category_id"],
            DEFAULT_CATEGORY_ID,
            "failed set must not fall back by rewriting"
        );
    });
}

#[test]
fn list_todo_categories_includes_default() {
    with_todo_task_sandbox(|_| {
        let listed = list_todo_categories();
        assert_eq!(listed["_status"], 200);
        let cats = listed["categories"].as_array().expect("categories");
        assert!(
            cats.iter().any(|c| {
                c["id"] == DEFAULT_CATEGORY_ID
                    && c["name"] == DEFAULT_CATEGORY_NAME
                    && c["is_default"] == true
            }),
            "list must include built-in 待分类"
        );
    });
}

#[test]
fn create_todo_category_persists_and_appears_in_list() {
    with_todo_task_sandbox(|_| {
        let created = create_todo_category("Research");
        assert_eq!(created["_status"], 201);
        let cat_id = created["category_id"].as_str().expect("id");
        assert!(!cat_id.is_empty());
        assert_eq!(created["category"]["name"], "Research");
        assert_eq!(created["category"]["is_default"], false);

        let listed = list_todo_categories();
        assert!(listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id && c["name"] == "Research"));
    });
}

#[test]
fn delete_empty_category_succeeds_and_disappears_from_list() {
    with_todo_task_sandbox(|_| {
        let created = create_todo_category("Ephemeral");
        let cat_id = created["category_id"].as_str().expect("id").to_string();
        let deleted = delete_todo_category(&cat_id);
        assert_eq!(deleted["_status"], 200);
        assert_eq!(deleted["ok"], true);

        let listed = list_todo_categories();
        assert!(!listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id));
    });
}

#[test]
fn delete_default_category_always_fails() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let deleted = delete_todo_category(DEFAULT_CATEGORY_ID);
        assert!(deleted["_status"].as_u64().unwrap_or(0) >= 400);
        assert!(deleted.get("error").is_some());

        let listed = list_todo_categories();
        assert!(listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == DEFAULT_CATEGORY_ID));
    });
}

#[test]
fn delete_non_empty_category_fails_without_reassigning_members() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let cat = create_todo_category("Occupied");
        let cat_id = cat["category_id"].as_str().expect("id").to_string();
        let created = create_master_with_category("Member", None, "", Some(&cat_id));
        let master_id = created["master_task_id"].as_str().expect("id");

        let deleted = delete_todo_category(&cat_id);
        assert!(deleted["_status"].as_u64().unwrap_or(0) >= 400);
        assert!(deleted.get("error").is_some());

        assert_eq!(get_by_id(master_id)["category_id"], cat_id);
        let listed = list_todo_categories();
        assert!(listed["categories"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == cat_id));
    });
}

/// T5 / SK-4 ops harness: default「待分类」→ temp category via set_master_category
/// (same Host path as MCP `update_todo_task` category_id). Optional evidence dump:
/// `T5_OPS_EVIDENCE_OUT=/abs/path.json`.
#[test]
fn t5_independent_reclassify_ops_demo() {
    with_todo_task_sandbox(|_| {
        ensure_default_category().expect("ensure");
        let created = create_master_with_category("T5 reclass sample", None, "", None);
        assert_eq!(created["_status"], 201);
        let master_id = created["master_task_id"].as_str().expect("id").to_string();
        assert_eq!(
            created["task"]["category_id"],
            DEFAULT_CATEGORY_ID,
            "sample starts in 待分类"
        );

        let cat = create_todo_category("T5 Ops Target");
        assert_eq!(cat["_status"], 201);
        let target_id = cat["category_id"].as_str().expect("id").to_string();
        let target_name = cat["category"]["name"].as_str().expect("name").to_string();

        let moved = set_master_category(&master_id, &target_id);
        assert_eq!(moved["_status"], 200);
        assert_eq!(moved["task"]["category_id"], target_id);
        assert_eq!(get_by_id(&master_id)["category_id"], target_id);

        let rejected = set_master_category(&master_id, "cat_invalid_t5_ops");
        assert!(rejected["_status"].as_u64().unwrap_or(0) >= 400);
        assert_eq!(
            get_by_id(&master_id)["category_id"],
            target_id,
            "invalid category_id must not rewrite disk"
        );

        let restored = set_master_category(&master_id, DEFAULT_CATEGORY_ID);
        assert_eq!(restored["_status"], 200);
        assert_eq!(get_by_id(&master_id)["category_id"], DEFAULT_CATEGORY_ID);

        let moved_again = set_master_category(&master_id, &target_id);
        assert_eq!(moved_again["_status"], 200);
        assert_eq!(get_by_id(&master_id)["category_id"], target_id);

        if let Ok(out) = std::env::var("T5_OPS_EVIDENCE_OUT") {
            let evidence = serde_json::json!({
                "task_id": "t5",
                "recorded_at": chrono::Utc::now().to_rfc3339(),
                "harness": "cargo test --lib t5_independent_reclassify_ops_demo (Host set_master_category; MCP update_todo_task category_id proxy)",
                "live_mcp_note": "IDE MCP schema at ops time lacked category_id; verified via worktree Host unit harness",
                "default_category": {
                    "id": DEFAULT_CATEGORY_ID,
                    "name": DEFAULT_CATEGORY_NAME,
                },
                "samples": [{
                    "master_task_id": master_id,
                    "title": "T5 reclass sample",
                    "from_category_id": DEFAULT_CATEGORY_ID,
                    "to_category_id": target_id,
                    "to_category_name": target_name,
                    "via": "set_master_category",
                    "status": "moved",
                }],
                "reversible": true,
                "invalid_category_rejected": true,
                "no_mcp_category_crud": true,
            });
            let path = Path::new(&out);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("evidence parent");
            }
            fs::write(
                path,
                serde_json::to_string_pretty(&evidence).expect("serialize evidence"),
            )
            .expect("write evidence");
        }
    });
}
