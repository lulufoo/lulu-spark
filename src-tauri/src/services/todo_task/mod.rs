//! Todo tasks persisted at `{workbench_knowledge_root}/plan_tasks/` (v2: index + per-task files).
//! Disk root rename to `todo_tasks/` is owned by a later task.

pub mod types;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use serde_json::{json, Value};

use crate::config::paths;
use crate::repositories::atomic_json;
use crate::services::id::random_hex12;

use types::{
    index_entry_task_dir, AttachmentEntry, AttachmentsFile, IndexEntry, MasterTask,
    MasterTaskStatus, PlanTasksIndex, SubTask, SubTaskStatus, SubTasksFile,
};

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
static TEST_FAIL_COMPLETE_SUB: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_FAIL_LINK_ARCHIVE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_FAIL_BATCH_SUB_TASKS: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_FAIL_BATCH_PLAN_MD: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_FAIL_BATCH_INDEX: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_FAIL_MIGRATE_IMPLICIT_WRITE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_FAIL_ADD_ATTACHMENT_MANIFEST: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_FAIL_DELETE_ATTACHMENT_FILE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub fn test_set_fail_complete_sub(fail: bool) {
    TEST_FAIL_COMPLETE_SUB.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_link_archive(fail: bool) {
    TEST_FAIL_LINK_ARCHIVE.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_batch_sub_tasks(fail: bool) {
    TEST_FAIL_BATCH_SUB_TASKS.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_batch_todo_md(fail: bool) {
    TEST_FAIL_BATCH_PLAN_MD.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_batch_index(fail: bool) {
    TEST_FAIL_BATCH_INDEX.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_migrate_implicit_write(fail: bool) {
    TEST_FAIL_MIGRATE_IMPLICIT_WRITE.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_add_attachment_manifest(fail: bool) {
    TEST_FAIL_ADD_ATTACHMENT_MANIFEST.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_delete_attachment_file(fail: bool) {
    TEST_FAIL_DELETE_ATTACHMENT_FILE.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_reset_all_injection_flags() {
    TEST_FAIL_COMPLETE_SUB.store(false, Ordering::SeqCst);
    TEST_FAIL_LINK_ARCHIVE.store(false, Ordering::SeqCst);
    TEST_FAIL_BATCH_SUB_TASKS.store(false, Ordering::SeqCst);
    TEST_FAIL_BATCH_PLAN_MD.store(false, Ordering::SeqCst);
    TEST_FAIL_BATCH_INDEX.store(false, Ordering::SeqCst);
    TEST_FAIL_MIGRATE_IMPLICIT_WRITE.store(false, Ordering::SeqCst);
    TEST_FAIL_ADD_ATTACHMENT_MANIFEST.store(false, Ordering::SeqCst);
    TEST_FAIL_DELETE_ATTACHMENT_FILE.store(false, Ordering::SeqCst);
}

#[cfg(test)]
fn test_take_fail_link_archive() -> bool {
    TEST_FAIL_LINK_ARCHIVE.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
fn test_take_fail_batch_sub_tasks() -> bool {
    TEST_FAIL_BATCH_SUB_TASKS.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
fn test_take_fail_batch_plan_md() -> bool {
    TEST_FAIL_BATCH_PLAN_MD.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
fn test_take_fail_batch_index() -> bool {
    TEST_FAIL_BATCH_INDEX.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
fn test_take_fail_migrate_implicit_write() -> bool {
    TEST_FAIL_MIGRATE_IMPLICIT_WRITE.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
fn test_take_fail_complete_sub() -> bool {
    TEST_FAIL_COMPLETE_SUB.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
fn test_take_fail_add_attachment_manifest() -> bool {
    TEST_FAIL_ADD_ATTACHMENT_MANIFEST.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
fn test_take_fail_delete_attachment_file() -> bool {
    TEST_FAIL_DELETE_ATTACHMENT_FILE.swap(false, Ordering::SeqCst)
}

const CORRUPT_STORAGE_ERROR: &str = "Invalid plan_tasks storage";

fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().expect("todo_task write lock");
    f()
}

fn delete_v1_if_present(path: &std::path::Path) -> Result<bool, String> {
    if path.is_file() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    Ok(false)
}

fn init_v2_index() -> Result<(), String> {
    let plan_dir = paths::plan_tasks_dir().map_err(|e| format!("{e:?}"))?;
    fs::create_dir_all(plan_dir.join("tasks")).map_err(|e| e.to_string())?;
    let index_path = paths::plan_tasks_index_path().map_err(|e| format!("{e:?}"))?;

    if index_path.is_file() {
        if let Ok(text) = fs::read_to_string(&index_path) {
            if let Ok(index) = serde_json::from_str::<PlanTasksIndex>(&text) {
                if index.version == 2 {
                    return Ok(());
                }
            }
        }
    }

    let index = PlanTasksIndex::default();
    write_plan_tasks_index(&index_path, &index)
}

fn write_plan_tasks_index(path: &std::path::Path, index: &PlanTasksIndex) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(index).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn ensure_bootstrap() -> Result<(), String> {
    let wb_v1 = paths::plan_tasks_path().map_err(|e| format!("{e:?}"))?;
    let cache_v1 = paths::cache_plan_tasks_v1_path().map_err(|e| format!("{e:?}"))?;

    let _deleted_wb = delete_v1_if_present(&wb_v1)?;
    let _deleted_cache = delete_v1_if_present(&cache_v1)?;

    init_v2_index()?;
    migrate_implicit_subs();
    Ok(())
}

fn migration_errors_path() -> Result<std::path::PathBuf, String> {
    paths::plan_tasks_dir()
        .map(|d| d.join("migration_errors.json"))
        .map_err(|e| format!("{e:?}"))
}

fn load_migration_errors_unlocked() -> HashMap<String, String> {
    let Ok(path) = migration_errors_path() else {
        return HashMap::new();
    };
    if !path.is_file() {
        return HashMap::new();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return HashMap::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_migration_errors(errors: &HashMap<String, String>) {
    let Ok(path) = migration_errors_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if errors.is_empty() {
        if path.is_file() {
            let _ = fs::remove_file(&path);
        }
        return;
    }
    if let Ok(text) = serde_json::to_string_pretty(errors) {
        let tmp = path.with_extension("json.tmp");
        if fs::write(&tmp, &text).is_ok() {
            let _ = fs::rename(&tmp, &path);
        }
    }
}

fn plan_has_migration_error(master_id: &str) -> bool {
    load_migration_errors_unlocked().contains_key(master_id)
}

fn write_migrated_sub_tasks(path: &Path, sub_tasks: &SubTasksFile) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_migrate_implicit_write() {
        return Err("injected migrate_implicit_subs write failure".to_string());
    }
    write_sub_tasks_json(path, sub_tasks)
}

fn migrate_one_plan_implicit_subs(master_id: &str) -> Result<(), String> {
    let path = paths::plan_tasks_sub_tasks_path(master_id).map_err(|e| format!("{e:?}"))?;
    if !path.is_file() {
        return Err("missing sub_tasks.json".to_string());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut file: SubTasksFile = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let before = file.sub_tasks.len();
    file.sub_tasks.retain(|s| !s.implicit);
    if file.sub_tasks.len() == before {
        return Ok(());
    }
    write_migrated_sub_tasks(&path, &file)
}

fn migrate_implicit_subs() {
    let index = match load_v2_index_unlocked() {
        Ok(i) => i,
        Err(_) => return,
    };
    let mut errors = load_migration_errors_unlocked();
    for master_id in index.tasks.keys() {
        match migrate_one_plan_implicit_subs(master_id) {
            Ok(()) => {
                errors.remove(master_id);
            }
            Err(message) => {
                errors.insert(master_id.clone(), message);
            }
        }
    }
    save_migration_errors(&errors);
}

fn bootstrap_error(err: String) -> Value {
    json!({ "error": err, "_status": 500 })
}

fn snapshot_index() -> Result<PlanTasksIndex, String> {
    let index_path = paths::plan_tasks_index_path().map_err(|e| format!("{e:?}"))?;
    if !index_path.is_file() {
        return Ok(PlanTasksIndex::default());
    }
    let text = fs::read_to_string(&index_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn write_plan_md_atomic(path: &Path, content: &str) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_batch_plan_md() {
        return Err("injected plan.md write failure".to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn write_sub_tasks_json(path: &Path, sub_tasks: &SubTasksFile) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_batch_sub_tasks() {
        return Err("injected sub_tasks.json write failure".to_string());
    }
    let value = serde_json::to_value(sub_tasks).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

fn write_index_snapshot(path: &Path, index: &PlanTasksIndex) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_batch_index() {
        return Err("injected index.json write failure".to_string());
    }
    if index.tasks.is_empty() {
        return write_plan_tasks_index(path, index);
    }
    let value = serde_json::to_value(index).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

fn rollback_before_index(master_task_id: &str) -> Result<(), String> {
    let task_dir = paths::plan_tasks_task_dir(master_task_id).map_err(|e| format!("{e:?}"))?;
    if task_dir.exists() {
        // AC8: whole task_dir cascade — includes attachments/ and attachments.json.
        fs::remove_dir_all(&task_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn rollback_index_failure(snapshot: &PlanTasksIndex) -> Result<(), String> {
    let index_path = paths::plan_tasks_index_path().map_err(|e| format!("{e:?}"))?;
    write_index_snapshot(&index_path, snapshot)
}

/// I-2 atomic batch: sub_tasks.json → plan.md → index entry. Caller must hold `WRITE_LOCK`.
fn write_task_batch(
    master_task_id: &str,
    index_entry: &IndexEntry,
    sub_tasks: &SubTasksFile,
    plan_md: &str,
) -> Result<(), String> {
    let snapshot = snapshot_index()?;
    let task_dir = paths::plan_tasks_task_dir(master_task_id).map_err(|e| format!("{e:?}"))?;
    fs::create_dir_all(&task_dir).map_err(|e| e.to_string())?;

    let sub_tasks_path = paths::plan_tasks_sub_tasks_path(master_task_id).map_err(|e| format!("{e:?}"))?;
    let plan_md_path = paths::plan_tasks_plan_md_path(master_task_id).map_err(|e| format!("{e:?}"))?;
    let index_path = paths::plan_tasks_index_path().map_err(|e| format!("{e:?}"))?;

    if let Err(e) = write_sub_tasks_json(&sub_tasks_path, sub_tasks) {
        let _ = rollback_before_index(master_task_id);
        return Err(e);
    }

    if let Err(e) = write_plan_md_atomic(&plan_md_path, plan_md) {
        let _ = rollback_before_index(master_task_id);
        return Err(e);
    }

    let mut index = snapshot.clone();
    index
        .tasks
        .insert(master_task_id.to_string(), index_entry.clone());

    if let Err(e) = write_index_snapshot(&index_path, &index) {
        let _ = rollback_index_failure(&snapshot);
        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
pub fn test_run_write_task_batch(
    master_task_id: &str,
    index_entry: &IndexEntry,
    sub_tasks: &SubTasksFile,
    plan_md: &str,
) -> Result<(), String> {
    with_write_lock(|| {
        ensure_bootstrap()?;
        write_task_batch(master_task_id, index_entry, sub_tasks, plan_md)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoadOutcome {
    Ok,
    Missing,
    Corrupt,
}

fn load_v2_index_unlocked() -> Result<PlanTasksIndex, LoadOutcome> {
    let index_path = match paths::plan_tasks_index_path() {
        Ok(p) => p,
        Err(_) => return Ok(PlanTasksIndex::default()),
    };
    if !index_path.is_file() {
        return Ok(PlanTasksIndex::default());
    }
    let text = match fs::read_to_string(&index_path) {
        Ok(t) => t,
        Err(_) => return Err(LoadOutcome::Corrupt),
    };
    match serde_json::from_str::<PlanTasksIndex>(&text) {
        Ok(index) if index.version == 2 => Ok(index),
        _ => Err(LoadOutcome::Corrupt),
    }
}

fn load_sub_tasks_file(master_id: &str) -> Result<SubTasksFile, LoadOutcome> {
    let path = match paths::plan_tasks_sub_tasks_path(master_id) {
        Ok(p) => p,
        Err(_) => return Err(LoadOutcome::Corrupt),
    };
    if !path.is_file() {
        return Err(LoadOutcome::Corrupt);
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return Err(LoadOutcome::Corrupt),
    };
    serde_json::from_str(&text).map_err(|_| LoadOutcome::Corrupt)
}

fn corrupt_storage_error() -> Value {
    json!({ "error": CORRUPT_STORAGE_ERROR, "_status": 500 })
}

fn load_v2_for_read() -> Result<PlanTasksIndex, Value> {
    if let Err(e) = ensure_bootstrap() {
        return Err(bootstrap_error(e));
    }
    match load_v2_index_unlocked() {
        Ok(index) => Ok(index),
        Err(LoadOutcome::Corrupt) => Err(corrupt_storage_error()),
        Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => Ok(PlanTasksIndex::default()),
    }
}

fn assemble_master_task(entry: &IndexEntry, subs: &SubTasksFile) -> MasterTask {
    MasterTask {
        master_task_id: entry.master_task_id.clone(),
        title: entry.title.clone(),
        status: entry.status.clone(),
        created_at: entry.created_at.clone(),
        sub_tasks: subs.sub_tasks.clone(),
    }
}

fn load_master_task_unlocked(master_id: &str) -> Result<MasterTask, Value> {
    let index = match load_v2_index_unlocked() {
        Ok(i) => i,
        Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
        Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => PlanTasksIndex::default(),
    };
    let entry = index
        .tasks
        .get(master_id)
        .ok_or_else(|| json!({ "error": "Task not found", "_status": 404 }))?;
    let subs = match load_sub_tasks_file(master_id) {
        Ok(s) => s,
        Err(LoadOutcome::Corrupt) | Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => {
            return Err(corrupt_storage_error());
        }
    };
    Ok(assemble_master_task(entry, &subs))
}

fn find_master_id_for_any_id(index: &PlanTasksIndex, id: &str) -> Result<Option<String>, Value> {
    if index.tasks.contains_key(id) {
        return Ok(Some(id.to_string()));
    }
    for master_id in index.tasks.keys() {
        let subs = match load_sub_tasks_file(master_id) {
            Ok(s) => s,
            Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => return Err(corrupt_storage_error()),
        };
        if subs.sub_tasks.iter().any(|s| s.sub_task_id == id) {
            return Ok(Some(master_id.clone()));
        }
    }
    Ok(None)
}

fn read_todo_md_or_empty(master_id: &str) -> Result<String, String> {
    let path = paths::plan_tasks_plan_md_path(master_id).map_err(|e| format!("{e:?}"))?;
    if !path.is_file() {
        return Ok(String::new());
    }
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

fn persist_master(master: &MasterTask) -> Result<(), String> {
    let plan_md = read_todo_md_or_empty(&master.master_task_id)?;
    persist_master_with_plan_md(master, &plan_md)
}

fn persist_master_with_plan_md(master: &MasterTask, plan_md: &str) -> Result<(), String> {
    let entry = IndexEntry {
        master_task_id: master.master_task_id.clone(),
        title: master.title.clone(),
        status: master.status.clone(),
        created_at: master.created_at.clone(),
        task_dir: index_entry_task_dir(&master.master_task_id),
    };
    let subs = SubTasksFile {
        sub_tasks: master.sub_tasks.clone(),
    };
    write_task_batch(&master.master_task_id, &entry, &subs, plan_md)
}

/// Count title units: each CJK ideograph = 1; each whitespace-delimited word token = 1.
/// Punctuation and other symbols do not count.
pub fn title_unit_count(title: &str) -> usize {
    let mut units = 0usize;
    let mut in_word = false;
    for c in title.chars() {
        if is_cjk_ideograph(c) {
            if in_word {
                units += 1;
                in_word = false;
            }
            units += 1;
        } else if c.is_whitespace() {
            if in_word {
                units += 1;
                in_word = false;
            }
        } else if c.is_ascii_alphanumeric()
            || matches!(c, '\u{00C0}'..='\u{024F}' | '\u{1E00}'..='\u{1EFF}')
        {
            in_word = true;
        } else if in_word {
            units += 1;
            in_word = false;
        }
    }
    if in_word {
        units += 1;
    }
    units
}

fn is_cjk_ideograph(c: char) -> bool {
    matches!(
        c,
        '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
            | '\u{2A700}'..='\u{2B73F}'
            | '\u{2B740}'..='\u{2B81F}'
            | '\u{2B820}'..='\u{2CEAF}'
            | '\u{2CEB0}'..='\u{2EBEF}'
            | '\u{30000}'..='\u{3134F}'
    )
}

const TITLE_UNIT_LIMIT: usize = 20;

fn new_master_id() -> String {
    format!("task_{}", random_hex12())
}

fn format_sub_id(master_id: &str, index: usize) -> String {
    format!("{master_id}_sub_{index:02}")
}

fn is_terminal_sub_status(status: &SubTaskStatus) -> bool {
    matches!(
        status,
        SubTaskStatus::Complete | SubTaskStatus::Abandoned
    )
}

fn master_to_value(master: &MasterTask) -> Value {
    serde_json::to_value(master).unwrap_or_else(|_| json!({}))
}

fn master_to_response(master: &MasterTask, migration_error: bool) -> Value {
    let todo_md = read_todo_md_or_empty(&master.master_task_id).unwrap_or_default();
    let mut value = master_to_value(master);
    if let Value::Object(ref mut map) = value {
        map.insert("todo_md".to_string(), json!(todo_md));
        map.insert("migration_error".to_string(), json!(migration_error));
    }
    value
}

fn load_master_response_unlocked(master_id: &str) -> Result<Value, Value> {
    let index = match load_v2_index_unlocked() {
        Ok(i) => i,
        Err(LoadOutcome::Corrupt) => return Err(corrupt_storage_error()),
        Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => PlanTasksIndex::default(),
    };
    let entry = index
        .tasks
        .get(master_id)
        .ok_or_else(|| json!({ "error": "Task not found", "_status": 404 }))?;
    let migration_error = plan_has_migration_error(master_id);
    if migration_error {
        let subs = match load_sub_tasks_file(master_id) {
            Ok(s) => s,
            Err(_) => SubTasksFile {
                sub_tasks: vec![],
            },
        };
        return Ok(master_to_response(
            &assemble_master_task(entry, &subs),
            true,
        ));
    }
    let subs = match load_sub_tasks_file(master_id) {
        Ok(s) => s,
        Err(LoadOutcome::Corrupt) | Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => {
            return Err(corrupt_storage_error());
        }
    };
    Ok(master_to_response(
        &assemble_master_task(entry, &subs),
        false,
    ))
}

pub fn read_todo_md(master_task_id: &str) -> Value {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }

    match load_v2_for_read() {
        Ok(index) => {
            if !index.tasks.contains_key(master_task_id) {
                return json!({ "error": "Task not found", "_status": 404 });
            }
            match read_todo_md_or_empty(master_task_id) {
                Ok(todo_md) => json!({ "todo_md": todo_md, "_status": 200 }),
                Err(e) => json!({ "error": e, "_status": 500 }),
            }
        }
        Err(err) => err,
    }
}

pub fn update_todo_md(master_task_id: &str, plan_md: &str) -> Value {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let index = match load_v2_index_unlocked() {
            Ok(i) => i,
            Err(LoadOutcome::Corrupt) => return corrupt_storage_error(),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => PlanTasksIndex::default(),
        };

        if !index.tasks.contains_key(master_task_id) {
            return json!({ "error": "Task not found", "_status": 404 });
        }

        let plan_md_path = match paths::plan_tasks_plan_md_path(master_task_id) {
            Ok(p) => p,
            Err(e) => return json!({ "error": format!("{e:?}"), "_status": 500 }),
        };

        match write_plan_md_atomic(&plan_md_path, plan_md) {
            Ok(()) => json!({ "ok": true, "_status": 200 }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn create_master_with_subs(title: &str, sub_titles: Option<&[&str]>) -> Value {
    create_master_with_subs_and_todo(title, sub_titles, "")
}

/// Create a master task. `plan_md` is written atomically with the task batch (empty → empty `plan.md`).
pub fn create_master_with_subs_and_todo(
    title: &str,
    sub_titles: Option<&[&str]>,
    plan_md: &str,
) -> Value {
    let title = title.trim();
    if title.is_empty() {
        return json!({ "error": "Missing title", "_status": 400 });
    }
    if title_unit_count(title) > TITLE_UNIT_LIMIT {
        return json!({
            "error": "Title too long (max 20 Chinese characters or English words)",
            "_status": 400
        });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        if let Err(LoadOutcome::Corrupt) = load_v2_index_unlocked() {
            return corrupt_storage_error();
        }

        let master_id = new_master_id();
        let created_at = Utc::now().to_rfc3339();
        let titles: Vec<String> = match sub_titles {
            Some(slice) if !slice.is_empty() => {
                for t in slice {
                    if t.trim().is_empty() {
                        return json!({ "error": "Invalid sub_titles element", "_status": 400 });
                    }
                }
                slice.iter().map(|t| t.trim().to_string()).collect()
            }
            _ => vec![],
        };

        let sub_tasks = if titles.is_empty() {
            vec![]
        } else {
            titles
                .iter()
                .enumerate()
                .map(|(i, t)| SubTask {
                    sub_task_id: format_sub_id(&master_id, i + 1),
                    title: Some(t.clone()),
                    status: SubTaskStatus::Incomplete,
                    implicit: false,
                    linked_archive_ids: vec![],
                    completed_at: None,
                })
                .collect()
        };

        let master = MasterTask {
            master_task_id: master_id.clone(),
            title: title.to_string(),
            status: MasterTaskStatus::Incomplete,
            created_at,
            sub_tasks,
        };

        match persist_master_with_plan_md(&master, plan_md) {
            Ok(()) => {
                let mut response = json!({
                    "master_task_id": master_id,
                    "task": master_to_value(&master),
                    "_status": 201,
                });
                if let Some(first_sub) = master.sub_tasks.first() {
                    response["sub_task_id"] = json!(first_sub.sub_task_id);
                }
                response
            }
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn get_by_id(id: &str) -> Value {
    let id = id.trim();
    if id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }

    match load_v2_for_read() {
        Ok(index) => match find_master_id_for_any_id(&index, id) {
            Ok(Some(master_id)) => match load_master_response_unlocked(&master_id) {
                Ok(response) => response,
                Err(err) => err,
            },
            Ok(None) => json!({ "error": "Task not found", "_status": 404 }),
            Err(err) => err,
        },
        Err(err) => err,
    }
}

pub fn list_all() -> Value {
    match load_v2_for_read() {
        Ok(index) => {
            let mut masters: Vec<Value> = Vec::new();
            for master_id in index.tasks.keys() {
                match load_master_response_unlocked(master_id) {
                    Ok(response) => masters.push(response),
                    Err(err) => return err,
                }
            }
            masters.sort_by(|a, b| {
                let a_ts = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                let b_ts = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                b_ts.cmp(a_ts)
            });
            Value::Array(masters)
        }
        Err(err) => err,
    }
}

pub fn delete_master(master_task_id: &str) -> Value {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let index = match load_v2_index_unlocked() {
            Ok(i) => i,
            Err(LoadOutcome::Corrupt) => return corrupt_storage_error(),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => PlanTasksIndex::default(),
        };

        if !index.tasks.contains_key(master_task_id) {
            return json!({ "error": "Task not found", "_status": 404 });
        }

        if let Err(e) = rollback_before_index(master_task_id) {
            return json!({ "error": e, "_status": 500 });
        }

        let index_path = match paths::plan_tasks_index_path() {
            Ok(p) => p,
            Err(e) => return json!({ "error": format!("{e:?}"), "_status": 500 }),
        };

        let mut updated = index;
        updated.tasks.remove(master_task_id);
        match write_index_snapshot(&index_path, &updated) {
            Ok(()) => json!({ "ok": true, "_status": 200 }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn add_sub(master_task_id: &str, title: &str) -> Value {
    let title = title.trim();
    if title.is_empty() {
        return json!({ "error": "Missing title", "_status": 400 });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        let next_index = master.sub_tasks.len() + 1;
        let new_sub = SubTask {
            sub_task_id: format_sub_id(master_task_id, next_index),
            title: Some(title.to_string()),
            status: SubTaskStatus::Incomplete,
            implicit: false,
            linked_archive_ids: vec![],
            completed_at: None,
        };
        let new_sub_id = new_sub.sub_task_id.clone();
        master.sub_tasks.push(new_sub);

        match persist_master(&master) {
            Ok(()) => json!({
                "sub_task_id": new_sub_id,
                "task": master_to_value(&master),
                "_status": 201,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn delete_sub(master_task_id: &str, sub_task_id: &str) -> Value {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        let pos = match master
            .sub_tasks
            .iter()
            .position(|s| s.sub_task_id == sub_task_id)
        {
            Some(p) => p,
            None => return json!({ "error": "Task not found", "_status": 404 }),
        };

        master.sub_tasks.remove(pos);

        match persist_master(&master) {
            Ok(()) => json!({
                "task": master_to_value(&master),
                "_status": 200,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn complete_sub(master_task_id: &str, sub_task_id: &str) -> Value {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        #[cfg(test)]
        if test_take_fail_complete_sub() {
            return json!({ "error": "injected todo_task failure", "_status": 500 });
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return json!({ "error": "Task not found", "_status": 404 });
        };

        if is_terminal_sub_status(&sub.status) {
            return json!({ "error": "Sub task is in terminal status", "_status": 409 });
        }

        sub.status = SubTaskStatus::Complete;
        sub.completed_at = Some(Utc::now().to_rfc3339());
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => json!({
                "task": master_to_value(&updated),
                "_status": 200,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn complete_todo(master_task_id: &str, sub_task_id: Option<&str>) -> Value {
    match sub_task_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(sub_id) => complete_sub(master_task_id, sub_id),
        None => complete_master(master_task_id),
    }
}

fn complete_master(master_task_id: &str) -> Value {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        match master.status {
            MasterTaskStatus::Complete => json!({
                "task": master_to_value(&master),
                "_status": 200,
            }),
            MasterTaskStatus::Abandoned => {
                json!({ "error": "master_abandoned", "_status": 409 })
            }
            MasterTaskStatus::Incomplete => {
                master.status = MasterTaskStatus::Complete;
                let updated = master.clone();
                match persist_master(&master) {
                    Ok(()) => json!({
                        "task": master_to_value(&updated),
                        "_status": 200,
                    }),
                    Err(e) => json!({ "error": e, "_status": 500 }),
                }
            }
        }
    })
}

pub fn update_sub_title(master_task_id: &str, sub_task_id: &str, title: &str) -> Value {
    let title = title.trim();
    if title.is_empty() {
        return json!({ "error": "Missing title", "_status": 400 });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return json!({ "error": "Task not found", "_status": 404 });
        };

        sub.title = Some(title.to_string());
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => json!({
                "task": master_to_value(&updated),
                "_status": 200,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn update_master_title(master_task_id: &str, title: &str) -> Value {
    let title = title.trim();
    if title.is_empty() {
        return json!({ "error": "Missing title", "_status": 400 });
    }
    if title_unit_count(title) > TITLE_UNIT_LIMIT {
        return json!({
            "error": "Title too long (max 20 Chinese characters or English words)",
            "_status": 400
        });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        master.title = title.to_string();
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => json!({
                "task": master_to_value(&updated),
                "_status": 200,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

fn parse_master_status_wire(status: &str) -> Option<MasterTaskStatus> {
    match status.trim() {
        "incomplete" => Some(MasterTaskStatus::Incomplete),
        "complete" => Some(MasterTaskStatus::Complete),
        "abandoned" => Some(MasterTaskStatus::Abandoned),
        _ => None,
    }
}

/// Host/UI-only explicit master status write (tri-state mutual transitions).
pub fn set_master_status(master_task_id: &str, status: &str) -> Value {
    let Some(next) = parse_master_status_wire(status) else {
        return json!({ "error": "Invalid status", "_status": 400 });
    };

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        master.status = next;
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => json!({
                "task": master_to_value(&updated),
                "_status": 200,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn abandon_sub(master_task_id: &str, sub_task_id: &str) -> Value {
    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return json!({ "error": "Task not found", "_status": 404 });
        };

        if is_terminal_sub_status(&sub.status) {
            return json!({ "error": "Sub task is in terminal status", "_status": 409 });
        }

        sub.status = SubTaskStatus::Abandoned;
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => json!({
                "task": master_to_value(&updated),
                "_status": 200,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

pub fn link_archive(master_task_id: &str, sub_task_id: &str, archive_id: &str) -> Value {
    let archive_id = archive_id.trim();
    if archive_id.is_empty() {
        return json!({ "error": "Missing archive id", "_status": 400 });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        #[cfg(test)]
        if test_take_fail_link_archive() {
            return json!({ "error": "injected link_archive failure", "_status": 500 });
        }

        let mut master = match load_master_task_unlocked(master_task_id) {
            Ok(m) => m,
            Err(err) => return err,
        };

        let Some(sub) = master
            .sub_tasks
            .iter_mut()
            .find(|s| s.sub_task_id == sub_task_id)
        else {
            return json!({ "error": "Task not found", "_status": 404 });
        };

        if !sub.linked_archive_ids.iter().any(|id| id == archive_id) {
            sub.linked_archive_ids.push(archive_id.to_string());
        }
        let updated = master.clone();

        match persist_master(&master) {
            Ok(()) => json!({
                "task": master_to_value(&updated),
                "_status": 200,
            }),
            Err(e) => json!({ "error": e, "_status": 500 }),
        }
    })
}

fn normalize_attachment_file_name(file_name: &str) -> Result<String, &'static str> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("Invalid file name");
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("Invalid file name");
    }
    let path = Path::new(trimmed);
    if path.components().count() != 1 {
        return Err("Invalid file name");
    }
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return Err("Invalid file name");
    };
    if name != trimmed {
        return Err("Invalid file name");
    }
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return Err("Only .md attachments are supported");
    };
    if !ext.eq_ignore_ascii_case("md") {
        return Err("Only .md attachments are supported");
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    if stem.is_empty() {
        return Err("Invalid file name");
    }
    Ok(name.to_string())
}

fn attachment_name_taken(dir: &Path, manifest: &AttachmentsFile, name: &str) -> bool {
    dir.join(name).exists() || manifest.attachments.iter().any(|e| e.file_name == name)
}

fn resolve_unique_attachment_name(
    attachments_dir: &Path,
    manifest: &AttachmentsFile,
    basename: &str,
) -> String {
    if !attachment_name_taken(attachments_dir, manifest, basename) {
        return basename.to_string();
    }
    let path = Path::new(basename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(basename);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("md");
    let mut n = 1u32;
    loop {
        let candidate = format!("{stem}-{n}.{ext}");
        if !attachment_name_taken(attachments_dir, manifest, &candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn load_attachments_file_unlocked(path: &Path) -> Result<AttachmentsFile, String> {
    if !path.is_file() {
        return Ok(AttachmentsFile {
            attachments: vec![],
        });
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn persist_attachments_manifest(path: &Path, file: &AttachmentsFile) -> Result<(), String> {
    let value = serde_json::to_value(file).map_err(|e| e.to_string())?;
    atomic_json::write_json(path, &value)
}

fn write_attachments_file(path: &Path, file: &AttachmentsFile) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_add_attachment_manifest() {
        return Err("injected attachments.json write failure".to_string());
    }
    persist_attachments_manifest(path, file)
}

fn restore_attachments_manifest(path: &Path, previous: &AttachmentsFile) -> Result<(), String> {
    if previous.attachments.is_empty() {
        if path.is_file() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    persist_attachments_manifest(path, previous)
}

fn remove_attachment_file(path: &Path) -> Result<(), String> {
    #[cfg(test)]
    if test_take_fail_delete_attachment_file() {
        return Err("injected attachment file delete failure".to_string());
    }
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

/// Copy a `.md` attachment into the plan task directory and append `attachments.json`.
/// On any post-copy failure, deletes the new file and restores the prior manifest.
pub fn add_attachment(master_task_id: &str, file_name: &str, content: &str) -> Value {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }

    let basename = match normalize_attachment_file_name(file_name) {
        Ok(name) => name,
        Err(msg) => return json!({ "error": msg, "_status": 400 }),
    };

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let index = match load_v2_index_unlocked() {
            Ok(i) => i,
            Err(LoadOutcome::Corrupt) => return corrupt_storage_error(),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => PlanTasksIndex::default(),
        };

        if !index.tasks.contains_key(master_task_id) {
            return json!({ "error": "Task not found", "_status": 404 });
        }

        let task_dir = match paths::plan_tasks_task_dir(master_task_id) {
            Ok(p) => p,
            Err(e) => return json!({ "error": format!("{e:?}"), "_status": 500 }),
        };
        let attachments_dir = task_dir.join("attachments");
        let manifest_path = task_dir.join("attachments.json");

        let previous = match load_attachments_file_unlocked(&manifest_path) {
            Ok(f) => f,
            Err(e) => return json!({ "error": e, "_status": 500 }),
        };

        let final_name =
            resolve_unique_attachment_name(&attachments_dir, &previous, &basename);
        let dest = attachments_dir.join(&final_name);

        if let Err(e) = fs::create_dir_all(&attachments_dir) {
            return json!({ "error": e.to_string(), "_status": 500 });
        }
        if let Err(e) = fs::write(&dest, content) {
            return json!({ "error": e.to_string(), "_status": 500 });
        }

        let entry = AttachmentEntry {
            file_name: final_name,
            original_file_name: basename,
            added_at: Utc::now().to_rfc3339(),
        };
        let mut updated = previous.clone();
        updated.attachments.push(entry);

        if let Err(e) = write_attachments_file(&manifest_path, &updated) {
            let _ = fs::remove_file(&dest);
            let _ = restore_attachments_manifest(&manifest_path, &previous);
            return json!({ "error": e, "_status": 500 });
        }

        let stored = updated.attachments.last().expect("just pushed");
        json!({
            "file_name": stored.file_name,
            "original_file_name": stored.original_file_name,
            "added_at": stored.added_at,
            "_status": 201,
        })
    })
}

fn attachment_in_manifest(manifest: &AttachmentsFile, file_name: &str) -> bool {
    manifest.attachments.iter().any(|e| e.file_name == file_name)
}

/// Task dir + `attachments.json` for an existing plan. Missing manifest → empty list.
fn load_plan_attachments_manifest(
    master_task_id: &str,
) -> Result<(std::path::PathBuf, AttachmentsFile), Value> {
    let task_dir = paths::plan_tasks_task_dir(master_task_id)
        .map_err(|e| json!({ "error": format!("{e:?}"), "_status": 500 }))?;
    let manifest = load_attachments_file_unlocked(&task_dir.join("attachments.json"))
        .map_err(|e| json!({ "error": e, "_status": 500 }))?;
    Ok((task_dir, manifest))
}

/// List attachments for a plan from `attachments.json` (SSOT; not a directory scan).
/// Missing manifest → empty collection. Unknown/unreadable plan → error.
pub fn list_attachments(master_task_id: &str) -> Value {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }

    match load_v2_for_read() {
        Ok(index) => {
            if !index.tasks.contains_key(master_task_id) {
                return json!({ "error": "Task not found", "_status": 404 });
            }
            match load_plan_attachments_manifest(master_task_id) {
                Ok((_, file)) => {
                    let attachments =
                        serde_json::to_value(&file.attachments).unwrap_or_else(|_| json!([]));
                    json!({ "attachments": attachments, "_status": 200 })
                }
                Err(err) => err,
            }
        }
        Err(err) => err,
    }
}

/// Read on-disk content of a manifest-listed attachment.
pub fn read_attachment(master_task_id: &str, file_name: &str) -> Value {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return json!({ "error": "Invalid file name", "_status": 400 });
    }

    match load_v2_for_read() {
        Ok(index) => {
            if !index.tasks.contains_key(master_task_id) {
                return json!({ "error": "Task not found", "_status": 404 });
            }
            let (task_dir, manifest) = match load_plan_attachments_manifest(master_task_id) {
                Ok(ctx) => ctx,
                Err(err) => return err,
            };
            if !attachment_in_manifest(&manifest, file_name) {
                return json!({ "error": "Attachment not found", "_status": 404 });
            }
            let path = task_dir.join("attachments").join(file_name);
            match fs::read_to_string(&path) {
                Ok(content) => json!({
                    "file_name": file_name,
                    "content": content,
                    "_status": 200,
                }),
                Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
            }
        }
        Err(err) => err,
    }
}

/// Overwrite on-disk content of a manifest-listed attachment. Does not modify `plan.md`.
pub fn save_attachment(master_task_id: &str, file_name: &str, content: &str) -> Value {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return json!({ "error": "Invalid file name", "_status": 400 });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let index = match load_v2_index_unlocked() {
            Ok(i) => i,
            Err(LoadOutcome::Corrupt) => return corrupt_storage_error(),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => PlanTasksIndex::default(),
        };

        if !index.tasks.contains_key(master_task_id) {
            return json!({ "error": "Task not found", "_status": 404 });
        }

        let (task_dir, manifest) = match load_plan_attachments_manifest(master_task_id) {
            Ok(ctx) => ctx,
            Err(err) => return err,
        };
        if !attachment_in_manifest(&manifest, file_name) {
            return json!({ "error": "Attachment not found", "_status": 404 });
        }

        let path = task_dir.join("attachments").join(file_name);
        match fs::write(&path, content) {
            Ok(()) => json!({ "ok": true, "_status": 200 }),
            Err(e) => json!({ "error": e.to_string(), "_status": 500 }),
        }
    })
}

/// Remove a manifest-listed attachment entry and its on-disk file together.
/// On file-delete failure, restores the prior manifest (no half-success).
pub fn delete_attachment(master_task_id: &str, file_name: &str) -> Value {
    let master_task_id = master_task_id.trim();
    if master_task_id.is_empty() {
        return json!({ "error": "Missing id", "_status": 400 });
    }
    let file_name = file_name.trim();
    if file_name.is_empty() {
        return json!({ "error": "Invalid file name", "_status": 400 });
    }

    with_write_lock(|| {
        if let Err(e) = ensure_bootstrap() {
            return bootstrap_error(e);
        }

        let index = match load_v2_index_unlocked() {
            Ok(i) => i,
            Err(LoadOutcome::Corrupt) => return corrupt_storage_error(),
            Err(LoadOutcome::Missing) | Err(LoadOutcome::Ok) => PlanTasksIndex::default(),
        };

        if !index.tasks.contains_key(master_task_id) {
            return json!({ "error": "Task not found", "_status": 404 });
        }

        let (task_dir, previous) = match load_plan_attachments_manifest(master_task_id) {
            Ok(ctx) => ctx,
            Err(err) => return err,
        };
        if !attachment_in_manifest(&previous, file_name) {
            return json!({ "error": "Attachment not found", "_status": 404 });
        }

        let manifest_path = task_dir.join("attachments.json");
        let path = task_dir.join("attachments").join(file_name);

        let mut updated = previous.clone();
        updated.attachments.retain(|e| e.file_name != file_name);

        // Manifest first, then file — on file failure restore prior manifest (IV-7).
        if let Err(e) = persist_attachments_manifest(&manifest_path, &updated) {
            return json!({ "error": e, "_status": 500 });
        }
        if let Err(e) = remove_attachment_file(&path) {
            let _ = restore_attachments_manifest(&manifest_path, &previous);
            return json!({ "error": e, "_status": 500 });
        }
        json!({ "ok": true, "_status": 200 })
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/todo_task.rs"]
mod tests;
