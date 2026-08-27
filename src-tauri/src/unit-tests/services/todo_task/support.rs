use super::*;
use crate::config::settings::{self, default_cache_dir};
use crate::services::todo_task::test_reset_all_injection_flags;
pub(super) use crate::test_support::TestSandbox;

pub(super) use std::fs;
pub(super) use std::path::Path;
pub(super) use std::time::SystemTime;
pub(super) use crate::config::paths;
pub(super) use crate::services::todo_task::types::{
    AttachmentsFile, CommentEntry, CommentsFile, IndexEntry, MasterTaskStatus, SubTask,
    SubTaskStatus, SubTasksFile, DEFAULT_CATEGORY_ID, DEFAULT_CATEGORY_NAME,
};

pub(super) const V1_STUB_JSON: &str = r#"{"version":1,"tasks":{}}"#;

pub(super) fn with_todo_task_sandbox<F: FnOnce(&Path)>(f: F) {
    let _sandbox = TestSandbox::new();
    test_reset_all_injection_flags();
    let wb = paths::workbench_knowledge_root().expect("workbench root");
    f(&wb);
}

pub(super) fn is_iso8601(s: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(s).is_ok()
}

pub(super) fn master_from_value(v: &serde_json::Value) -> &serde_json::Value {
    v.get("task").expect("task field")
}

pub(super) fn prod_todo_tasks_mtime() -> Option<SystemTime> {
    settings::AppSettings::default()
        .workbench_knowledge_root
        .join("todo_tasks")
        .join("todo_tasks.json")
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
}

pub(super) fn prod_cache_todo_tasks_mtime() -> Option<SystemTime> {
    default_cache_dir()
        .join("todo_tasks.json")
        .metadata()
        .ok()
        .and_then(|m| m.modified().ok())
}

pub(super) fn seed_v1_file(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir v1 parent");
    }
    fs::write(path, V1_STUB_JSON).expect("write v1 stub");
}

pub(super) fn read_index_version(wb: &Path) -> u32 {
    let index_path = wb.join("todo_tasks").join("index.json");
    let text = fs::read_to_string(&index_path).expect("read index.json");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse index.json");
    parsed["version"].as_u64().expect("index version") as u32
}

pub(super) fn sample_index_entry(master_id: &str) -> IndexEntry {
    IndexEntry {
        master_task_id: master_id.to_string(),
        title: "Batch task".to_string(),
        status: MasterTaskStatus::Incomplete,
        created_at: "2026-07-08T00:00:00+00:00".to_string(),
        task_dir: format!("tasks/{master_id}"),
        category_id: DEFAULT_CATEGORY_ID.to_string(),
    }
}

pub(super) fn sample_sub_tasks(master_id: &str) -> SubTasksFile {
    SubTasksFile {
        sub_tasks: vec![SubTask {
            sub_task_id: format!("{master_id}_sub_01"),
            title: Some("Batch task".to_string()),
            content: None,
            status: SubTaskStatus::Incomplete,
            implicit: true,
            linked_archive_ids: vec![],
            completed_at: None,
        }],
    }
}

pub(super) fn seed_v2_plan_with_subs(
    wb: &Path,
    master_id: &str,
    title: &str,
    sub_tasks: &serde_json::Value,
    merge_index: bool,
) {
    let todo_tasks_dir = wb.join("todo_tasks");
    fs::create_dir_all(todo_tasks_dir.join("tasks").join(master_id)).expect("mkdir task");
    if merge_index {
        let index_path = todo_tasks_dir.join("index.json");
        let mut index: serde_json::Value = if index_path.is_file() {
            serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap_or_else(|_| {
                serde_json::json!({ "version": 2, "tasks": {} })
            })
        } else {
            fs::create_dir_all(todo_tasks_dir.join("tasks")).expect("mkdir tasks");
            serde_json::json!({ "version": 2, "tasks": {} })
        };
        index["tasks"][master_id] = serde_json::json!({
            "master_task_id": master_id,
            "title": title,
            "status": "incomplete",
            "created_at": "2026-07-08T00:00:00+00:00",
            "task_dir": format!("tasks/{master_id}")
        });
        fs::write(
            index_path,
            serde_json::to_string_pretty(&index).expect("serialize index"),
        )
        .expect("write index");
    }
    fs::write(
        todo_tasks_dir
            .join("tasks")
            .join(master_id)
            .join("sub_tasks.json"),
        serde_json::to_string_pretty(sub_tasks).expect("serialize subs"),
    )
    .expect("write sub_tasks");
    fs::write(
        todo_tasks_dir
            .join("tasks")
            .join(master_id)
            .join("todo.md"),
        "",
    )
    .expect("write plan.md");
}

pub(super) fn subs_on_disk(wb: &Path, master_id: &str) -> serde_json::Value {
    let path = wb
        .join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("sub_tasks.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).expect("parse sub_tasks.json")
}

pub(super) fn attachments_dir(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("attachments")
}

pub(super) fn attachments_json_path(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("attachments.json")
}

pub(super) fn load_attachments_file(wb: &Path, master_id: &str) -> AttachmentsFile {
    let text = fs::read_to_string(attachments_json_path(wb, master_id)).expect("attachments.json");
    serde_json::from_str(&text).expect("parse attachments.json")
}

pub(super) fn write_attach_source(name: &str, content: impl AsRef<[u8]>) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "todo_attach_src_{}_{}",
        std::process::id(),
        n
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path.canonicalize().unwrap_or(path)
}

pub(super) fn add_from(master_id: &str, name: &str, content: impl AsRef<[u8]>) -> serde_json::Value {
    let path = write_attach_source(name, content);
    add_attachment(master_id, path.to_str().unwrap())
}

pub(super) fn save_from(
    master_id: &str,
    file_name: &str,
    source_name: &str,
    content: impl AsRef<[u8]>,
) -> serde_json::Value {
    let path = write_attach_source(source_name, content);
    save_attachment(master_id, file_name, path.to_str().unwrap())
}

pub(super) fn comments_json_abs_path(wb: &Path, master_id: &str) -> std::path::PathBuf {
    wb.join("todo_tasks")
        .join("tasks")
        .join(master_id)
        .join("comments.json")
}
