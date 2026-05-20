use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::meili_env::kb_root_string;
use crate::services::workbench_read;

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
    pub workbench_job: Arc<Mutex<JobState>>,
}

impl ReindexState {
    pub fn new() -> Self {
        Self {
            knowledge_job: Arc::new(Mutex::new(JobState::idle())),
            workbench_job: Arc::new(Mutex::new(JobState::idle())),
        }
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn run_python_script(repo_root: &Path, script: &str, extra_args: &[&str]) -> Result<String, String> {
    let script_path = repo_root.join("scripts").join(script);
    let output = Command::new("python3")
        .arg(&script_path)
        .args(extra_args)
        .current_dir(repo_root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(if err.is_empty() {
            if out.is_empty() {
                "未知错误".to_string()
            } else {
                out
            }
        } else {
            err
        });
    }
    let log = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if log.is_empty() { "完成".to_string() } else { log })
}

pub fn run_workbench_reindex_blocking(repo_root: &Path) -> Result<String, String> {
    run_python_script(repo_root, "build_workbench_index.py", &["--wipe"])
}

pub fn sync_repo_blocking(repo: &str, kb_root: &Path) -> (String, String, Option<String>) {
    let repo_name = repo.split('/').next_back().unwrap_or(repo).to_string();
    let local_dir = kb_root.join(&repo_name);
    if !local_dir.is_dir() {
        let output = Command::new("gh")
            .args(["repo", "clone", repo, local_dir.to_string_lossy().as_ref()])
            .output();
        match output {
            Ok(o) if o.status.success() => (repo_name, "cloned".to_string(), None),
            Ok(o) => {
                let _ = fs::remove_dir_all(&local_dir);
                let err = String::from_utf8_lossy(&o.stderr)
                    .trim()
                    .to_string();
                let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
                (
                    repo_name,
                    "clone_failed".to_string(),
                    Some(if err.is_empty() { out } else { err }),
                )
            }
            Err(e) => (
                repo_name,
                "clone_failed".to_string(),
                Some(e.to_string()),
            ),
        }
    } else {
        let output = Command::new("git")
            .args([
                "-C",
                local_dir.to_string_lossy().as_ref(),
                "pull",
                "--rebase",
            ])
            .output();
        match output {
            Ok(o) if o.status.success() => (repo_name, "pulled".to_string(), None),
            Ok(o) => {
                let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
                (
                    repo_name,
                    "pull_failed".to_string(),
                    Some(if err.is_empty() { out } else { err }),
                )
            }
            Err(e) => (
                repo_name,
                "pull_failed".to_string(),
                Some(e.to_string()),
            ),
        }
    }
}

pub fn run_knowledge_reindex_blocking(
    repo_root: &Path,
    slot: Option<&Arc<Mutex<JobState>>>,
) -> Result<String, String> {
    if let Some(s) = slot {
        set_job_log(s, "同步仓库（并行）…");
    }
    let topics = workbench_read::get_topics(repo_root);
    let repos: Vec<String> = topics
        .get("topics")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("repo").and_then(|r| r.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let kb_root = PathBuf::from(kb_root_string(repo_root));
    let mut failed_repos = Vec::new();
    let handles: Vec<_> = repos
        .iter()
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
    let mut log = run_python_script(repo_root, "build_knowledge_index.py", &[])?;
    if !failed_repos.is_empty() {
        log.push_str("\n⚠ 同步失败：");
        log.push_str(&failed_repos.join("；"));
    }
    Ok(log)
}

pub fn clear_repo_commit_cache(repo_root: &Path, repo_name: &str) -> Result<(), String> {
    let cache_file = repo_root.join(".cache").join("repo-commits.json");
    if !cache_file.is_file() {
        return Ok(());
    }
    let text = fs::read_to_string(&cache_file).map_err(|e| e.to_string())?;
    let mut cache: serde_json::Map<String, Value> =
        serde_json::from_str(&text).unwrap_or_else(|_| serde_json::Map::new());
    if cache.remove(repo_name).is_some() {
        let out = serde_json::to_string_pretty(&cache).map_err(|e| e.to_string())?;
        fs::write(cache_file, out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn run_kb_reindex_blocking(repo_root: &Path, repo: &str) -> Result<String, String> {
    let repo = repo.trim();
    if repo.is_empty() || !repo.contains('/') {
        return Err("invalid repo format".to_string());
    }
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let kb_root = PathBuf::from(kb_root_string(repo_root));
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return Err(format!("repo not cloned locally: {repo_name}"));
    }
    clear_repo_commit_cache(repo_root, repo_name)?;
    run_python_script(repo_root, "build_knowledge_index.py", &[])
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
mod tests {
    use super::*;

    #[test]
    fn job_state_idle_has_expected_fields() {
        let j = JobState::idle();
        assert_eq!(j.status, "idle");
        assert!(j.started_at.is_none());
        assert!(j.finished_at.is_none());
        assert!(j.log.is_empty());
        let v = j.to_json();
        assert_eq!(v["status"], "idle");
        assert!(v.get("log").is_some());
    }
}
