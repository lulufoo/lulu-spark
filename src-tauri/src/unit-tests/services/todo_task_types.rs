use std::collections::HashMap;

use serde_json::{json, Value};

use crate::services::todo_task::types::{
    attachments_json_rel_path, comments_json_rel_path, index_entry_task_dir, AttachmentEntry,
    AttachmentsFile, CommentEntry, CommentsFile, IndexEntry, MasterTaskStatus, PlanTasksIndex,
    SubTask, SubTaskStatus, SubTasksFile,
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
            content: None,
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
        content: None,
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
        content: None,
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
        content: None,
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
fn sub_task_status_serializes_three_lowercase_variants() {
    for (status, expected) in [
        (SubTaskStatus::Incomplete, "incomplete"),
        (SubTaskStatus::Complete, "complete"),
        (SubTaskStatus::Abandoned, "abandoned"),
    ] {
        let sub = SubTask {
            sub_task_id: "task_abc_sub_01".to_string(),
            title: Some("Sub".to_string()),
            content: None,
            status,
            implicit: false,
            linked_archive_ids: vec![],
            completed_at: None,
        };
        let v: Value = serde_json::to_value(&sub).expect("serialize");
        assert_eq!(v["status"], expected);
    }
}

#[test]
fn master_task_status_serializes_three_lowercase_variants() {
    for (status, expected) in [
        (MasterTaskStatus::Incomplete, "incomplete"),
        (MasterTaskStatus::Complete, "complete"),
        (MasterTaskStatus::Abandoned, "abandoned"),
    ] {
        let v: Value = serde_json::to_value(&status).expect("serialize");
        assert_eq!(v, expected);
        let parsed: MasterTaskStatus = serde_json::from_value(v).expect("deserialize");
        assert_eq!(parsed, status);
    }
}

#[test]
fn master_task_status_abandoned_roundtrip() {
    let raw = json!("abandoned");
    let status: MasterTaskStatus = serde_json::from_value(raw).expect("deserialize");
    assert_eq!(status, MasterTaskStatus::Abandoned);
    let v: Value = serde_json::to_value(&status).expect("serialize");
    assert_eq!(v, "abandoned");
}

#[test]
fn master_task_status_deserialize_unknown_rejects() {
    let result: Result<MasterTaskStatus, _> = serde_json::from_value(json!("not_a_status"));
    assert!(result.is_err());
}

#[test]
fn index_entry_missing_status_defaults_to_incomplete() {
    let raw = json!({
        "master_task_id": "task_abc",
        "title": "Legacy",
        "created_at": "2026-07-08T00:00:00+00:00",
        "task_dir": "tasks/task_abc"
    });
    let entry: IndexEntry = serde_json::from_value(raw).expect("deserialize");
    assert_eq!(entry.status, MasterTaskStatus::Incomplete);
}

#[test]
fn index_entry_explicit_complete_is_not_defaulted_away() {
    let raw = json!({
        "master_task_id": "task_abc",
        "title": "Done",
        "status": "complete",
        "created_at": "2026-07-08T00:00:00+00:00",
        "task_dir": "tasks/task_abc"
    });
    let entry: IndexEntry = serde_json::from_value(raw).expect("deserialize");
    assert_eq!(entry.status, MasterTaskStatus::Complete);
}

#[test]
fn sub_task_status_abandoned_roundtrip() {
    let raw = json!({
        "sub_task_id": "task_abc_sub_01",
        "title": "Sub",
        "status": "abandoned",
        "implicit": false,
        "linked_archive_ids": []
    });
    let sub: SubTask = serde_json::from_value(raw).expect("deserialize");
    assert_eq!(sub.status, SubTaskStatus::Abandoned);
}

#[test]
fn sub_task_deserialize_unknown_status_rejects() {
    let raw = json!({
        "sub_task_id": "task_abc_sub_01",
        "status": "not_a_status",
        "implicit": false
    });
    let result: Result<SubTask, _> = serde_json::from_value(raw);
    assert!(result.is_err());
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

#[test]
fn attachments_json_rel_path_is_under_task_dir() {
    let master_id = "task_a1b2c3d4e5f6";
    let path = attachments_json_rel_path(master_id);
    assert_eq!(path, format!("tasks/{master_id}/attachments.json"));
    assert!(!path.starts_with('/'));
    assert!(!std::path::Path::new(&path).is_absolute());
}

#[test]
fn attachment_entry_serializes_stored_original_and_added_at() {
    let entry = AttachmentEntry {
        file_name: "notes-1.md".to_string(),
        original_file_name: "notes.md".to_string(),
        added_at: "2026-07-18T00:00:00+00:00".to_string(),
    };
    let v: Value = serde_json::to_value(&entry).expect("serialize");
    assert_eq!(v["file_name"], "notes-1.md");
    assert_eq!(v["original_file_name"], "notes.md");
    assert_eq!(v["added_at"], "2026-07-18T00:00:00+00:00");
}

#[test]
fn attachments_file_roundtrip_preserves_entries() {
    let file = AttachmentsFile {
        attachments: vec![AttachmentEntry {
            file_name: "notes.md".to_string(),
            original_file_name: "notes.md".to_string(),
            added_at: "2026-07-18T00:00:00+00:00".to_string(),
        }],
    };
    let text = serde_json::to_string(&file).expect("serialize");
    let parsed: AttachmentsFile = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.attachments.len(), 1);
    assert_eq!(parsed.attachments[0].file_name, "notes.md");
    assert_eq!(parsed.attachments[0].original_file_name, "notes.md");
    assert_eq!(parsed.attachments[0].added_at, "2026-07-18T00:00:00+00:00");
}

#[test]
fn attachments_file_empty_roundtrip() {
    let file = AttachmentsFile {
        attachments: vec![],
    };
    let text = serde_json::to_string(&file).expect("serialize");
    let parsed: AttachmentsFile = serde_json::from_str(&text).expect("deserialize");
    assert!(parsed.attachments.is_empty());
    let v: Value = serde_json::from_str(&text).expect("value");
    assert!(v.get("attachments").and_then(|a| a.as_array()).is_some());
    assert!(v["attachments"].as_array().unwrap().is_empty());
}

#[test]
fn attachments_file_invalid_json_returns_error() {
    let result: Result<AttachmentsFile, _> = serde_json::from_str("{not valid json");
    assert!(result.is_err());
}

#[test]
fn index_entry_does_not_include_attachments_field() {
    let entry = IndexEntry {
        master_task_id: "task_abc".to_string(),
        title: "Example".to_string(),
        status: MasterTaskStatus::Incomplete,
        created_at: "2026-07-08T00:00:00+00:00".to_string(),
        task_dir: index_entry_task_dir("task_abc"),
    };
    let v: Value = serde_json::to_value(&entry).expect("serialize");
    assert!(v.get("attachments").is_none());
    assert!(v.get("attachments.json").is_none());
}

#[test]
fn comments_json_rel_path_is_under_task_dir() {
    let master_id = "task_a1b2c3d4e5f6";
    let path = comments_json_rel_path(master_id);
    assert_eq!(path, format!("tasks/{master_id}/comments.json"));
    assert!(!path.starts_with('/'));
    assert!(!std::path::Path::new(&path).is_absolute());
}

#[test]
fn comment_entry_serializes_id_body_created_at_only() {
    let entry = CommentEntry {
        id: "cmt_abcdef012345".to_string(),
        body: "decision note".to_string(),
        created_at: "2026-07-23T00:00:00+00:00".to_string(),
    };
    let v: Value = serde_json::to_value(&entry).expect("serialize");
    assert_eq!(v["id"], "cmt_abcdef012345");
    assert_eq!(v["body"], "decision note");
    assert_eq!(v["created_at"], "2026-07-23T00:00:00+00:00");
    assert!(v.get("updated_at").is_none());
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    assert_eq!(keys.len(), 3);
}

#[test]
fn comments_file_roundtrip_preserves_entries() {
    let file = CommentsFile {
        comments: vec![CommentEntry {
            id: "cmt_abcdef012345".to_string(),
            body: "note".to_string(),
            created_at: "2026-07-23T00:00:00+00:00".to_string(),
        }],
    };
    let text = serde_json::to_string(&file).expect("serialize");
    let parsed: CommentsFile = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.comments.len(), 1);
    assert_eq!(parsed.comments[0].id, "cmt_abcdef012345");
    assert_eq!(parsed.comments[0].body, "note");
    assert_eq!(parsed.comments[0].created_at, "2026-07-23T00:00:00+00:00");
}

#[test]
fn comments_file_empty_roundtrip_keeps_empty_array_semantics() {
    let file = CommentsFile {
        comments: vec![],
    };
    let text = serde_json::to_string(&file).expect("serialize");
    let parsed: CommentsFile = serde_json::from_str(&text).expect("deserialize");
    assert!(parsed.comments.is_empty());
    let v: Value = serde_json::from_str(&text).expect("value");
    assert!(v.get("comments").and_then(|a| a.as_array()).is_some());
    assert!(v["comments"].as_array().unwrap().is_empty());
}

#[test]
fn comments_file_rejects_bare_array_and_invalid_json() {
    let bare: Result<CommentsFile, _> = serde_json::from_str(
        r#"[{"id":"cmt_x","body":"a","created_at":"2026-07-23T00:00:00+00:00"}]"#,
    );
    assert!(bare.is_err());
    let bad: Result<CommentsFile, _> = serde_json::from_str("{not valid json");
    assert!(bad.is_err());
}

#[test]
fn comments_file_has_no_count_cap_in_envelope() {
    let comments: Vec<CommentEntry> = (0..64)
        .map(|i| CommentEntry {
            id: format!("cmt_{i:012x}"),
            body: format!("body-{i}"),
            created_at: "2026-07-23T00:00:00+00:00".to_string(),
        })
        .collect();
    let file = CommentsFile { comments };
    let text = serde_json::to_string(&file).expect("serialize");
    let parsed: CommentsFile = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.comments.len(), 64);
}

#[test]
fn index_entry_does_not_include_comments_field() {
    let entry = IndexEntry {
        master_task_id: "task_abc".to_string(),
        title: "Example".to_string(),
        status: MasterTaskStatus::Incomplete,
        created_at: "2026-07-08T00:00:00+00:00".to_string(),
        task_dir: index_entry_task_dir("task_abc"),
    };
    let v: Value = serde_json::to_value(&entry).expect("serialize");
    assert!(v.get("comments").is_none());
    assert!(v.get("comments.json").is_none());
}

#[test]
fn sub_task_missing_content_deserializes_as_none() {
    let raw = json!({
        "sub_task_id": "task_abc_sub_01",
        "title": "Legacy",
        "status": "incomplete",
        "implicit": false,
        "linked_archive_ids": []
    });
    let sub: SubTask = serde_json::from_value(raw).expect("deserialize legacy without content");
    assert_eq!(sub.content, None);
}

#[test]
fn sub_task_content_roundtrip_preserves_value() {
    let sub = SubTask {
        sub_task_id: "task_abc_sub_01".to_string(),
        title: Some("Sub".to_string()),
        content: Some("optional body".to_string()),
        status: SubTaskStatus::Incomplete,
        implicit: false,
        linked_archive_ids: vec![],
        completed_at: None,
    };
    let text = serde_json::to_string(&sub).expect("serialize");
    let parsed: SubTask = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(parsed.content.as_deref(), Some("optional body"));
}

#[test]
fn sub_task_content_none_omitted_from_json() {
    let sub = SubTask {
        sub_task_id: "task_abc_sub_01".to_string(),
        title: Some("Sub".to_string()),
        content: None,
        status: SubTaskStatus::Incomplete,
        implicit: false,
        linked_archive_ids: vec![],
        completed_at: None,
    };
    let v: Value = serde_json::to_value(&sub).expect("serialize");
    assert!(v.get("content").is_none());
}

#[test]
fn sub_task_content_empty_string_omitted_from_json() {
    let sub = SubTask {
        sub_task_id: "task_abc_sub_01".to_string(),
        title: Some("Sub".to_string()),
        content: Some(String::new()),
        status: SubTaskStatus::Incomplete,
        implicit: false,
        linked_archive_ids: vec![],
        completed_at: None,
    };
    let v: Value = serde_json::to_value(&sub).expect("serialize");
    assert!(v.get("content").is_none());
}

#[test]
fn sub_tasks_file_invalid_json_and_non_object_entry_still_fail() {
    let invalid: Result<SubTasksFile, _> = serde_json::from_str("{not valid json");
    assert!(invalid.is_err());

    let non_object_entry = json!({
        "sub_tasks": ["not-an-object"]
    });
    let bad_entry: Result<SubTasksFile, _> = serde_json::from_value(non_object_entry);
    assert!(bad_entry.is_err());
}
