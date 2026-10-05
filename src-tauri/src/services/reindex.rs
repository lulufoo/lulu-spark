use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::services::index_build::{rebuild_knowledge_index, rebuild_spark_index};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobState {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    pub log: String,
}

impl JobState {
    pub fn idle() -> Self {
        Self {
            status: "idle".to_string(),
            started_at: None,
            finished_at: None,
            log: String::new(),
        }
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({}))
    }
}

pub struct ReindexState {
    pub knowledge_job: Arc<Mutex<JobState>>,
    pub spark_job: Arc<Mutex<JobState>>,
}

impl ReindexState {
    pub fn new() -> Self {
        Self {
            knowledge_job: Arc::new(Mutex::new(JobState::idle())),
            spark_job: Arc::new(Mutex::new(JobState::idle())),
        }
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

/// Index-only rebuild of notes then knowledge (no `git pull`). Drives both job
/// slots so the header control and any per-slot poller see the same state.
/// Used by app startup and the header "Rebuild index" control.
pub fn run_all_reindex_blocking(
    repo_root: &Path,
    spark_slot: &Arc<Mutex<JobState>>,
    knowledge_slot: &Arc<Mutex<JobState>>,
) -> Result<(), String> {
    start_job(spark_slot, "Indexing notes…")?;
    if let Err(e) = start_job(knowledge_slot, "Waiting for notes…") {
        finish_job_error(spark_slot, e.clone());
        return Err(e);
    }
    match rebuild_spark_index(repo_root) {
        Ok(log) => finish_job_success(spark_slot, log),
        Err(e) => {
            finish_job_error(spark_slot, e.clone());
            finish_job_error(knowledge_slot, "Skipped: notes index failed".to_string());
            return Err(e);
        }
    }
    set_job_log(knowledge_slot, "Indexing knowledge…");
    match rebuild_knowledge_index(repo_root, None) {
        Ok(log) => {
            finish_job_success(knowledge_slot, log);
            Ok(())
        }
        Err(e) => {
            finish_job_error(knowledge_slot, e.clone());
            Err(e)
        }
    }
}

/// Combined view of both slots for the single header control.
pub fn all_status_json(
    spark_slot: &Arc<Mutex<JobState>>,
    knowledge_slot: &Arc<Mutex<JobState>>,
) -> Result<Value, String> {
    let wb = spark_slot.lock().map_err(|e| e.to_string())?.clone();
    let kb = knowledge_slot.lock().map_err(|e| e.to_string())?.clone();
    let status = if wb.status == "running" || kb.status == "running" {
        "running"
    } else if wb.status == "error" || kb.status == "error" {
        "error"
    } else if wb.status == "done" && kb.status == "done" {
        "done"
    } else {
        "idle"
    };
    let log = match status {
        "running" => {
            if wb.status == "running" {
                wb.log.clone()
            } else {
                kb.log.clone()
            }
        }
        _ => format!("Notes: {}\nKnowledge: {}", wb.log, kb.log),
    };
    Ok(json!({
        "status": status,
        "log": log,
        "spark": wb.to_json(),
        "knowledge": kb.to_json(),
    }))
}

pub fn run_kb_sync_and_index_blocking(repo_root: &Path, repo: &str) -> Result<String, String> {
    let repo = repo.trim();
    if repo.is_empty() {
        return Err("invalid directory name".to_string());
    }
    rebuild_knowledge_index(repo_root, Some(repo))
}

pub fn start_job(
    slot: &Arc<Mutex<JobState>>,
    initial_log: &str,
) -> Result<(), String> {
    let mut job = slot.lock().map_err(|e| e.to_string())?;
    if job.status == "running" {
        return Err("already running".to_string());
    }
    *job = JobState {
        status: "running".to_string(),
        started_at: Some(now_iso()),
        finished_at: None,
        log: initial_log.to_string(),
    };
    Ok(())
}

pub fn finish_job_success(slot: &Arc<Mutex<JobState>>, log: String) {
    if let Ok(mut job) = slot.lock() {
        job.status = "done".to_string();
        job.log = log;
        job.finished_at = Some(now_iso());
    }
}

pub fn finish_job_error(slot: &Arc<Mutex<JobState>>, log: String) {
    if let Ok(mut job) = slot.lock() {
        job.status = "error".to_string();
        job.log = log;
        job.finished_at = Some(now_iso());
    }
}

pub fn set_job_log(slot: &Arc<Mutex<JobState>>, log: impl Into<String>) {
    if let Ok(mut job) = slot.lock() {
        job.log = log.into();
    }
}

#[cfg(test)]
#[path = "../unit-tests/services/reindex.rs"]
mod tests;
