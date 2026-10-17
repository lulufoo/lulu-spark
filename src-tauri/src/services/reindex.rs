use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::services::index_build::rebuild_spark_index;

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
    pub spark_job: Arc<Mutex<JobState>>,
}

impl ReindexState {
    pub fn new() -> Self {
        Self {
            spark_job: Arc::new(Mutex::new(JobState::idle())),
        }
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

/// Index-only rebuild of every workspace's documents (no `git pull`).
/// Used by app startup and the header "Rebuild index" control.
pub fn run_all_reindex_blocking(
    repo_root: &Path,
    spark_slot: &Arc<Mutex<JobState>>,
) -> Result<(), String> {
    start_job(spark_slot, "Indexing documents…")?;
    match rebuild_spark_index(repo_root) {
        Ok(log) => {
            finish_job_success(spark_slot, log);
            Ok(())
        }
        Err(e) => {
            finish_job_error(spark_slot, e.clone());
            Err(e)
        }
    }
}

/// Status of the single header control.
pub fn all_status_json(spark_slot: &Arc<Mutex<JobState>>) -> Result<Value, String> {
    let job = spark_slot.lock().map_err(|e| e.to_string())?.clone();
    Ok(json!({
        "status": job.status,
        "log": job.log,
        "spark": job.to_json(),
    }))
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
