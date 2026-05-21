use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::meili_env::{knowledge_corpus_root_string, workbench_knowledge_root_path};
use crate::config::paths;
use crate::integrations::git;
use crate::services::index_build::{rebuild_knowledge_index, rebuild_workbench_index};
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
    pub repo_list_job: Arc<Mutex<JobState>>,
}

impl ReindexState {
    pub fn new() -> Self {
        Self {
            knowledge_job: Arc::new(Mutex::new(JobState::idle())),
            workbench_job: Arc::new(Mutex::new(JobState::idle())),
            repo_list_job: Arc::new(Mutex::new(JobState::idle())),
        }
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn run_workbench_reindex_blocking(repo_root: &Path) -> Result<String, String> {
    rebuild_workbench_index(repo_root)
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
        match crate::integrations::git::pull_rebase(&local_dir) {
            Ok(o) if o.success => (repo_name, "pulled".to_string(), None),
            Ok(o) => {
                let err = o.stderr.trim().to_string();
                let out = o.stdout.trim().to_string();
                (
                    repo_name,
                    "pull_failed".to_string(),
                    Some(if err.is_empty() { out } else { err }),
                )
            }
            Err(e) => (
                repo_name,
                "pull_failed".to_string(),
                Some(e.message),
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

    let kb_root = PathBuf::from(knowledge_corpus_root_string(repo_root));
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
    let mut log = rebuild_knowledge_index(repo_root, false, None)?;
    if !failed_repos.is_empty() {
        log.push_str("\n⚠ 同步失败：");
        log.push_str(&failed_repos.join("；"));
    }
    Ok(log)
}

/// Sync (clone/pull) a WORKBENCH_KNOWLEDGE repo, then rebuild the workbench index.
/// Used by the per-repo SYNC button in the WORKBENCH_KNOWLEDGE list.
pub fn run_wb_sync_and_index_blocking(repo_root: &Path, repo: &str) -> Result<String, String> {
    let repo = repo.trim();
    if repo.is_empty() || !repo.contains('/') {
        return Err("invalid repo format".to_string());
    }
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let wb_root = workbench_knowledge_root_path(repo_root);
    let synced_via_archive = wb_root
        .file_name()
        .and_then(|n| n.to_str())
        .map(|dir_name| dir_name == repo_name)
        .unwrap_or(false)
        && wb_root.join(".git").exists();
    if synced_via_archive {
        match git::pull_rebase(&wb_root) {
            Ok(o) if o.success => {}
            Ok(o) => {
                return Err(format!(
                    "git pull failed for {repo_name}: {}",
                    o.stderr.trim()
                ));
            }
            Err(e) => return Err(format!("git pull failed for {repo_name}: {e}")),
        }
    } else {
        let kb_root = PathBuf::from(knowledge_corpus_root_string(repo_root));
        let (_, status, err) = sync_repo_blocking(repo, &kb_root);
        if status.contains("failed") {
            let msg = err.unwrap_or_else(|| "unknown error".to_string());
            return Err(format!("git sync failed for {repo_name}: {msg}"));
        }
    }
    rebuild_workbench_index(repo_root)
}

/// Pull all locally-cloned WORKBENCH_KNOWLEDGE repos, then rebuild the workbench index.
/// Used by the bulk 全量同步 button in the WORKBENCH_KNOWLEDGE list view.
pub fn run_workbench_pull_and_reindex(
    repo_root: &Path,
    slot: Option<&Arc<Mutex<JobState>>>,
) -> Result<String, String> {
    if let Some(s) = slot {
        set_job_log(s, "拉取 workbench 仓库…");
    }
    // Collect WORKBENCH_KNOWLEDGE repos from repo-list.json
    let repos: Vec<String> = {
        let cache_path = paths::repo_list_cache_path().map_err(|e| format!("{e:?}"))?;
        if cache_path.is_file() {
            let text = fs::read_to_string(&cache_path).unwrap_or_default();
            serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("repos").and_then(|r| r.as_array()).cloned())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|item| {
                            if item.get("type").and_then(|v| v.as_str())
                                != Some("WORKBENCH_KNOWLEDGE")
                            {
                                return None;
                            }
                            item.get("full_name").and_then(|v| v.as_str()).map(String::from)
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    };

    let wb_root = workbench_knowledge_root_path(repo_root);
    let mut failed_repos = Vec::new();
    if wb_root.join(".git").exists() {
        match git::pull_rebase(&wb_root) {
            Ok(o) if !o.success => {
                failed_repos.push(format!(
                    "workbench archive: {}",
                    o.stderr.trim()
                ));
            }
            Err(e) => failed_repos.push(format!("workbench archive: {e}")),
            _ => {}
        }
    } else {
        let kb_root = PathBuf::from(knowledge_corpus_root_string(repo_root));
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
    }

    if let Some(s) = slot {
        set_job_log(s, "重建 workbench 索引…");
    }
    let mut log = rebuild_workbench_index(repo_root)?;
    if !failed_repos.is_empty() {
        log.push_str("\n⚠ 拉取失败：");
        log.push_str(&failed_repos.join("；"));
    }
    Ok(log)
}

pub fn run_kb_sync_and_index_blocking(repo_root: &Path, repo: &str) -> Result<String, String> {
    let repo = repo.trim();
    if repo.is_empty() || !repo.contains('/') {
        return Err("invalid repo format".to_string());
    }
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let kb_root = PathBuf::from(knowledge_corpus_root_string(repo_root));
    let (_, status, err) = sync_repo_blocking(repo, &kb_root);
    if status.contains("failed") {
        let msg = err.unwrap_or_else(|| "unknown error".to_string());
        return Err(format!("git sync failed for {repo_name}: {msg}"));
    }
    clear_repo_commit_cache(repo_root, repo_name)?;
    rebuild_knowledge_index(repo_root, false, Some(repo))
}

pub fn run_knowledge_pull_and_reindex(
    repo_root: &Path,
    slot: Option<&Arc<Mutex<JobState>>>,
) -> Result<String, String> {
    if let Some(s) = slot {
        set_job_log(s, "拉取本地仓库（并行）…");
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

    let kb_root = PathBuf::from(knowledge_corpus_root_string(repo_root));
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
    let mut log = rebuild_knowledge_index(repo_root, false, None)?;
    if !failed_repos.is_empty() {
        log.push_str("\n⚠ 拉取失败：");
        log.push_str(&failed_repos.join("；"));
    }
    Ok(log)
}

pub fn clear_repo_commit_cache(repo_root: &Path, repo_name: &str) -> Result<(), String> {
    let _ = repo_root;
    let cache_file = paths::cache_dir()
        .map_err(|e| format!("{e:?}"))?
        .join("repo-commits.json");
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
    let kb_root = PathBuf::from(knowledge_corpus_root_string(repo_root));
    let local_dir = kb_root.join(repo_name);
    if !local_dir.is_dir() {
        return Err(format!("repo not cloned locally: {repo_name}"));
    }
    clear_repo_commit_cache(repo_root, repo_name)?;
    rebuild_knowledge_index(repo_root, false, Some(repo))
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
