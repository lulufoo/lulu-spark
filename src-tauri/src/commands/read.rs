use serde_json::{json, Value};
use tauri::{AppHandle, State};

use crate::config::paths;
use crate::integrations::{gh_read, meilisearch};
use crate::services::reindex::{finish_job_error, finish_job_success, start_job, ReindexState};
use crate::services::workbench_read;

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub async fn search_knowledge(
    _app: AppHandle,
    q: String,
    limit: Option<u32>,
) -> Result<Value, String> {
    let root = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || {
        meilisearch::search_json(&root, "knowledge", &q, limit)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_workbench(
    _app: AppHandle,
    q: String,
    limit: Option<u32>,
) -> Result<Value, String> {
    let root = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || {
        meilisearch::search_json(&root, "workbench", &q, limit)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_topics(_app: AppHandle) -> Result<Value, String> {
    Ok(workbench_read::get_topics(&repo_root()?))
}

#[tauri::command]
pub fn get_annotations(_app: AppHandle) -> Result<Value, String> {
    Ok(workbench_read::get_annotations_summary(&repo_root()?))
}

#[tauri::command]
pub fn get_annotation(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(workbench_read::get_annotation(&repo_root()?, &path))
}

#[tauri::command]
pub fn get_draft(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(workbench_read::get_draft(&repo_root()?, &path))
}

#[tauri::command]
pub fn get_config(_app: AppHandle) -> Result<Value, String> {
    Ok(workbench_read::get_config(&repo_root()?))
}

#[tauri::command]
pub async fn get_status(_app: AppHandle) -> Result<Value, String> {
    let root = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || workbench_read::get_status(&root))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn kb_read(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    Ok(crate::services::kb::kb_read_json(&repo_root()?, &repo, &path))
}

#[tauri::command]
pub fn kb_annotation(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    Ok(crate::services::kb::kb_annotation_json(&repo_root()?, &repo, &path))
}

#[tauri::command]
pub fn kb_status(_app: AppHandle, repo: String) -> Result<Value, String> {
    Ok(crate::services::kb::kb_status_json(&repo_root()?, &repo))
}

/// When `force=false`: reads from disk cache immediately (non-blocking).
/// When `force=true`: spawns a background job that refreshes from GitHub,
///   returns `{"status":"running"}`. Poll via `get_repo_list_status`.
#[tauri::command]
pub fn get_repo_list(
    _app: AppHandle,
    state: State<'_, ReindexState>,
    force: Option<bool>,
) -> Result<Value, String> {
    let root = repo_root()?;
    if !force.unwrap_or(false) {
        return Ok(gh_read::repo_list_json(&root, false));
    }

    // If already running, just return running — don't double-start.
    {
        let job = state.repo_list_job.lock().map_err(|e| e.to_string())?;
        if job.status == "running" {
            return Ok(json!({ "status": "running" }));
        }
    }

    start_job(&state.repo_list_job, "查询 GitHub 仓库列表…")?;
    let slot = state.repo_list_job.clone();
    let repo_root = root.clone();
    tauri::async_runtime::spawn(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            gh_read::repo_list_json(&repo_root, true)
        })
        .await;
        match result {
            Ok(v) => {
                if v.get("error").is_some() {
                    finish_job_error(
                        &slot,
                        v["error"].as_str().unwrap_or("unknown error").to_string(),
                    );
                } else {
                    finish_job_success(&slot, "仓库列表已更新".to_string());
                }
            }
            Err(e) => finish_job_error(&slot, e.to_string()),
        }
    });
    Ok(json!({ "status": "running" }))
}

#[tauri::command]
pub fn get_repo_list_status(state: State<'_, ReindexState>) -> Result<Value, String> {
    let job = state.repo_list_job.lock().map_err(|e| e.to_string())?;
    Ok(job.to_json())
}

#[tauri::command]
pub async fn get_repo_dirs(_app: AppHandle, repo: String) -> Result<Value, String> {
    let root = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || gh_read::repo_dirs_json(&root, &repo))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_file(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    let root = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || gh_read::check_file_json(&root, &repo, &path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_link_title(_app: AppHandle, url: String) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::services::link_title::fetch_link_title(&url)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_corpus_index(_app: AppHandle) -> Result<Value, String> {
    Ok(workbench_read::get_corpus_index(&repo_root()?))
}

#[tauri::command]
pub fn get_corpus_file(
    _app: AppHandle,
    layer: String,
    path: String,
) -> Result<Value, String> {
    Ok(workbench_read::get_corpus_file(&repo_root()?, &layer, &path))
}

/// Return every WORKBENCH_KNOWLEDGE repo (derived from repo-list) together with
/// a `local_exists` flag indicating whether the repo is already cloned locally.
/// `filter_type` defaults to `KNOWLEDGE_CORPUS`; pass `WORKBENCH_KNOWLEDGE` for the
/// workbench corpus repos.
#[tauri::command]
pub fn get_kb_corpus_status(
    _app: AppHandle,
    filter_type: Option<String>,
) -> Result<Value, String> {
    let repo_root = repo_root()?;
    let filter = filter_type.as_deref().unwrap_or("KNOWLEDGE_CORPUS");
    let kb_root = std::path::PathBuf::from(
        crate::config::meili_env::kb_root_string(&repo_root),
    );

    let repos: Vec<Value> = if filter == "KNOWLEDGE_CORPUS" {
        // Fast path: use the already-filtered topics list
        let topics = workbench_read::get_topics(&repo_root);
        topics
            .get("topics")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let full_name = item.get("repo")?.as_str()?;
                        let name = full_name.split('/').next_back().unwrap_or(full_name);
                        let local_exists = kb_root.join(name).is_dir();
                        Some(json!({
                            "full_name": full_name,
                            "name": name,
                            "description": item.get("description").cloned().unwrap_or(Value::Null),
                            "local_exists": local_exists,
                        }))
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        // Read repo-list.json and filter by the requested type
        let cache_path = crate::config::paths::repo_list_cache_path()
            .map_err(|e| format!("{e:?}"))?;
        if !cache_path.is_file() {
            return Ok(json!({ "repos": [] }));
        }
        let text = std::fs::read_to_string(&cache_path).map_err(|e| e.to_string())?;
        let data: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        data.get("repos")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|repo| {
                        if repo.get("type").and_then(|v| v.as_str()) != Some(filter) {
                            return None;
                        }
                        let full_name = repo.get("full_name")?.as_str()?;
                        let name = full_name.split('/').next_back().unwrap_or(full_name);
                        let local_exists = kb_root.join(name).is_dir();
                        Some(json!({
                            "full_name": full_name,
                            "name": name,
                            "description": repo.get("description").cloned().unwrap_or(Value::Null),
                            "local_exists": local_exists,
                        }))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    Ok(json!({ "repos": repos }))
}
