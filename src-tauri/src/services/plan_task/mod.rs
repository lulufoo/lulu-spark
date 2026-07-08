//! Plan tasks persisted at `{workbench_knowledge_root}/plan_tasks/` (v2: index + per-task files).

pub mod types;

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
    index_entry_task_dir, IndexEntry, MasterTask, MasterTaskStatus, PlanTasksIndex, SubTask,
    SubTaskStatus, SubTasksFile,
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
pub fn test_set_fail_batch_plan_md(fail: bool) {
    TEST_FAIL_BATCH_PLAN_MD.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_batch_index(fail: bool) {
    TEST_FAIL_BATCH_INDEX.store(fail, Ordering::SeqCst);
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
fn test_take_fail_complete_sub() -> bool {
    TEST_FAIL_COMPLETE_SUB.swap(false, Ordering::SeqCst)
}

const CORRUPT_STORAGE_ERROR: &str = "Invalid plan_tasks storage";

fn with_write_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let _guard = WRITE_LOCK.lock().expect("plan_task write lock");
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

    init_v2_index()
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
    let mut master = MasterTask {
        master_task_id: entry.master_task_id.clone(),
        title: entry.title.clone(),
        status: entry.status.clone(),
        created_at: entry.created_at.clone(),
        sub_tasks: subs.sub_tasks.clone(),
    };
    recompute_master_status(&mut master);
    master
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

fn load_master_task(master_id: &str) -> Result<MasterTask, Value> {
    load_v2_for_read()?;
    load_master_task_unlocked(master_id)
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

fn read_plan_md_or_empty(master_id: &str) -> Result<String, String> {
    let path = paths::plan_tasks_plan_md_path(master_id).map_err(|e| format!("{e:?}"))?;
    if !path.is_file() {
        return Ok(String::new());
    }
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

fn persist_master(master: &MasterTask) -> Result<(), String> {
    let mut master = master.clone();
    recompute_master_status(&mut master);
    let plan_md = read_plan_md_or_empty(&master.master_task_id)?;
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
    write_task_batch(&master.master_task_id, &entry, &subs, &plan_md)
}

fn new_master_id() -> String {
    format!("task_{}", random_hex12())
}

fn format_sub_id(master_id: &str, index: usize) -> String {
    format!("{master_id}_sub_{index:02}")
}

fn recompute_master_status(master: &mut MasterTask) {
    let all_complete = master
        .sub_tasks
        .iter()
        .all(|s| s.status == SubTaskStatus::Complete);
    master.status = if all_complete {
        MasterTaskStatus::Complete
    } else {
        MasterTaskStatus::Incomplete
    };
}

fn master_to_value(master: &MasterTask) -> Value {
    serde_json::to_value(master).unwrap_or_else(|_| json!({}))
}

pub fn create_master_with_subs(title: &str, sub_titles: Option<&[&str]>) -> Value {
    let title = title.trim();
    if title.is_empty() {
        return json!({ "error": "Missing title", "_status": 400 });
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
            vec![SubTask {
                sub_task_id: format_sub_id(&master_id, 1),
                title: Some(title.to_string()),
                status: SubTaskStatus::Incomplete,
                implicit: true,
                linked_archive_ids: vec![],
                completed_at: None,
            }]
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

        let first_sub_id = sub_tasks[0].sub_task_id.clone();
        let master = MasterTask {
            master_task_id: master_id.clone(),
            title: title.to_string(),
            status: MasterTaskStatus::Incomplete,
            created_at,
            sub_tasks,
        };

        match persist_master(&master) {
            Ok(()) => json!({
                "master_task_id": master_id,
                "sub_task_id": first_sub_id,
                "task": master_to_value(&master),
                "_status": 201,
            }),
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
            Ok(Some(master_id)) => match load_master_task(&master_id) {
                Ok(master) => master_to_value(&master),
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
                match load_master_task(master_id) {
                    Ok(master) => masters.push(master_to_value(&master)),
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
        recompute_master_status(&mut master);

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

        if master.sub_tasks.len() <= 1 {
            return json!({ "error": "Cannot delete last sub", "_status": 400 });
        }

        let pos = match master
            .sub_tasks
            .iter()
            .position(|s| s.sub_task_id == sub_task_id)
        {
            Some(p) => p,
            None => return json!({ "error": "Task not found", "_status": 404 }),
        };

        master.sub_tasks.remove(pos);
        recompute_master_status(&mut master);

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
            return json!({ "error": "injected plan_task failure", "_status": 500 });
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

        sub.status = SubTaskStatus::Complete;
        sub.completed_at = Some(Utc::now().to_rfc3339());
        recompute_master_status(&mut master);
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

#[cfg(test)]
#[path = "../../unit-tests/services/plan_task.rs"]
mod tests;
