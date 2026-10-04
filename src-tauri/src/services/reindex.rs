use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::roots::knowledge_root_string;
use crate::integrations::git;
use crate::services::index_build::{rebuild_knowledge_index, rebuild_spark_index};
use crate::services::spark_read;

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

pub fn sync_repo_blocking(repo: &str, kb_root: &Path) -> (String, String, Option<String>) {
    let repo_name = repo.split('/').next_back().unwrap_or(repo).to_string();
    let local_dir = kb_root.join(&repo_name);
    if !local_dir.is_dir() {
        match crate::integrations::git::clone_repo(repo, &local_dir) {
            Ok(o) if o.success => (repo_name, "cloned".to_string(), None),
            Ok(o) => {
                let _ = fs::remove_dir_all(&local_dir);
                let err = o.stderr.trim().to_string();
                let out = o.stdout.trim().to_string();
                (
                    repo_name,
                    "clone_failed".to_string(),
                    Some(if err.is_empty() { out } else { err }),
                )
            }
            Err(e) => (
                repo_name,
                "clone_failed".to_string(),
                Some(e.message),
            ),
        }
    } else {
        let format_output = |stdout: &str, stderr: &str| {
            let err = stderr.trim().to_string();
            let out = stdout.trim().to_string();
            if err.is_empty() { out } else { err }
        };

        let (stash_empty, stash_out) = match git::stash_save(&local_dir) {
            Ok(result) => result,
            Err(e) => {
                return (
                    repo_name,
                    "pull_failed".to_string(),
                    Some(e.message),
                )
            }
        };

        if !stash_empty && !stash_out.success {
            return (
                repo_name,
                "pull_failed".to_string(),
                Some(format_output(&stash_out.stdout, &stash_out.stderr)),
            );
        }

        match crate::integrations::git::pull_rebase(&local_dir) {
            Ok(o) if o.success => {
                if stash_empty {
                    (repo_name, "pulled".to_string(), None)
                } else {
                    match git::stash_pop(&local_dir) {
                        Ok(pop) if pop.success => (repo_name, "pulled".to_string(), None),
                        Ok(pop) => (
                            repo_name,
                            "pull_failed".to_string(),
                            Some(format!(
                                "stash pop failed: {}",
                                format_output(&pop.stdout, &pop.stderr)
                            )),
                        ),
                        Err(e) => (
                            repo_name,
                            "pull_failed".to_string(),
                            Some(e.message),
                        ),
                    }
                }
            }
            Ok(o) => {
                if !stash_empty {
                    let _ = git::stash_pop(&local_dir);
                }
                (
                    repo_name,
                    "pull_failed".to_string(),
                    Some(format_output(&o.stdout, &o.stderr)),
                )
            }
            Err(e) => {
                if !stash_empty {
                    let _ = git::stash_pop(&local_dir);
                }
                (
                    repo_name,
                    "pull_failed".to_string(),
                    Some(e.message),
                )
            }
        }
    }
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
    if repo.is_empty() || !repo.contains('/') {
        return Err("invalid repo format".to_string());
    }
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let kb_root = PathBuf::from(knowledge_root_string(repo_root));
    let (_, status, err) = sync_repo_blocking(repo, &kb_root);
    if status.contains("failed") {
        let msg = err.unwrap_or_else(|| "unknown error".to_string());
        return Err(format!("git sync failed for {repo_name}: {msg}"));
    }
    rebuild_knowledge_index(repo_root, Some(repo))
}

pub fn run_knowledge_pull_and_reindex(
    repo_root: &Path,
    slot: Option<&Arc<Mutex<JobState>>>,
) -> Result<String, String> {
    if let Some(s) = slot {
        set_job_log(s, "拉取本地仓库（并行）…");
    }
    let topics = spark_read::get_topics(repo_root);
    let repos: Vec<String> = topics
        .get("topics")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("repo").and_then(|r| r.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let kb_root = PathBuf::from(knowledge_root_string(repo_root));
    let mut failed_repos = Vec::new();
    let handles: Vec<_> = repos
        .iter()
        .filter(|repo| {
            let name = repo.split('/').next_back().unwrap_or(repo);
            kb_root.join(name).is_dir()
        })
        .map(|repo| {
            let repo = repo.clone();
            let kb = kb_root.clone();
            thread::spawn(move || sync_repo_blocking(&repo, &kb))
        })
        .collect();
    for handle in handles {
        if let Ok((name, status, err)) = handle.join() {
            if status.contains("failed") {
                failed_repos.push(format!("{name}: {}", err.unwrap_or_default()));
            }
        }
    }

    if let Some(s) = slot {
        set_job_log(s, "重建索引…");
    }
    let mut log = rebuild_knowledge_index(repo_root, None)?;
    if !failed_repos.is_empty() {
        log.push_str("\n⚠ 拉取失败：");
        log.push_str(&failed_repos.join("；"));
    }
    Ok(log)
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
