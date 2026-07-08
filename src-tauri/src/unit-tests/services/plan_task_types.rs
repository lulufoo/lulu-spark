use std::collections::HashMap;

use serde_json::{json, Value};

use crate::services::plan_task::types::{
    index_entry_task_dir, IndexEntry, MasterTaskStatus, PlanTasksIndex, SubTask, SubTaskStatus,
    SubTasksFile,
};

#[test]
fn plan_tasks_index_default_is_version_two_with_empty_tasks() {
    let index = PlanTasksIndex::default();
    assert_eq!(index.version, 2);
    assert!(index.tasks.is_empty());
}

#[test]
fn index_entry_task_dir_is_relative_tasks_prefix() {
    let master_id = "task_a1b2c3d4e5f6";
    let task_dir = index_entry_task_dir(master_id);
    assert_eq!(task_dir, format!("tasks/{master_id}"));
    assert!(!task_dir.starts_with('/'));
    assert!(!std::path::Path::new(&task_dir).is_absolute());
}

#[test]
fn index_entry_serializes_tech_doc_fields() {
    let entry = IndexEntry {
        master_task_id: "task_abc".to_string(),
        title: "Example".to_string(),
        status: MasterTaskStatus::Incomplete,
        created_at: "2026-07-08T00:00:00+00:00".to_string(),
        task_dir: index_entry_task_dir("task_abc"),
    };
    let v: Value = serde_json::to_value(&entry).expect("serialize");
    assert_eq!(v["master_task_id"], "task_abc");
    assert_eq!(v["title"], "Example");
    assert_eq!(v["status"], "incomplete");
    assert_eq!(v["created_at"], "2026-07-08T00:00:00+00:00");
    assert_eq!(v["task_dir"], "tasks/task_abc");
}

#[test]
fn sub_tasks_file_wraps_sub_tasks_array() {
    let file = SubTasksFile {
        sub_tasks: vec![SubTask {
            sub_task_id: "task_abc_sub_01".to_string(),
            title: Some("Sub".to_string()),
            status: SubTaskStatus::Incomplete,
            implicit: false,
            linked_archive_ids: vec![],
            completed_at: None,
        }],
    };
    let v: Value = serde_json::to_value(&file).expect("serialize");
    assert!(v.get("sub_tasks").and_then(|v| v.as_array()).is_some());
}

#[test]
fn sub_task_title_none_omitted_from_json() {
    let sub = SubTask {
        sub_task_id: "task_abc_sub_01".to_string(),
        title: None,
        status: SubTaskStatus::Incomplete,
        implicit: true,
        linked_archive_ids: vec![],
        completed_at: None,
    };
    let v: Value = serde_json::to_value(&sub).expect("serialize");
    assert!(v.get("title").is_none());
}

#[test]
fn sub_task_completed_at_omitted_when_incomplete() {
    let sub = SubTask {
        sub_task_id: "task_abc_sub_01".to_string(),
        title: Some("Sub".to_string()),
        status: SubTaskStatus::Incomplete,
        implicit: false,
        linked_archive_ids: vec![],
        completed_at: None,
    };
    let v: Value = serde_json::to_value(&sub).expect("serialize");
    assert!(v.get("completed_at").is_none());
}

#[test]
fn sub_task_complete_includes_completed_at() {
    let sub = SubTask {
        sub_task_id: "task_abc_sub_01".to_string(),
        title: Some("Sub".to_string()),
        status: SubTaskStatus::Complete,
        implicit: false,
        linked_archive_ids: vec![],
        completed_at: Some("2026-07-08T01:00:00+00:00".to_string()),
    };
    let v: Value = serde_json::to_value(&sub).expect("serialize");
    assert_eq!(v["completed_at"], "2026-07-08T01:00:00+00:00");
}

#[test]
fn sub_task_linked_archive_ids_default_empty_array() {
    let raw = json!({
        "sub_task_id": "task_abc_sub_01",
        "status": "incomplete",
        "implicit": true
    });
    let sub: SubTask = serde_json::from_value(raw).expect("deserialize");
    assert!(sub.linked_archive_ids.is_empty());
}

#[test]
fn plan_tasks_index_roundtrip_preserves_tasks_map() {
    let mut tasks = HashMap::new();
    tasks.insert(
        "task_abc".to_string(),
        IndexEntry {
            master_task_id: "task_abc".to_string(),
            title: "Example".to_string(),
            status: MasterTaskStatus::Complete,
            created_at: "2026-07-08T00:00:00+00:00".to_string(),
            task_dir: index_entry_task_dir("task_abc"),
        },
    );
    let index = PlanTasksIndex {
        version: 2,
        tasks,
    };
    let text = serde_json::to_string(&index).expect("serialize");
    let parsed: PlanTasksIndex = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.version, 2);
    assert_eq!(parsed.tasks.len(), 1);
    assert_eq!(parsed.tasks["task_abc"].status, MasterTaskStatus::Complete);
}
