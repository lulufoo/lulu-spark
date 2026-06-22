use serde_json::{json, Value};
use tauri::{AppHandle, State};

use crate::config::paths;
use crate::integrations::{gh_read, meilisearch};
use crate::services::reindex::{finish_job_error, finish_job_success, start_job, ReindexState};
use crate::config::meili_env::workbench_knowledge_root_path;
use crate::services::sediment_kb;
use crate::services::tags_registry;
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
pub fn get_tags_registry(_app: AppHandle) -> Result<Value, String> {
    let corpus = workbench_knowledge_root_path(&repo_root()?);
    Ok(tags_registry::read_registry(&corpus))
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
pub fn infer_github_user_url(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(workbench_read::infer_github_user_url(&path))
}

#[tauri::command]
pub fn check_workbench_knowledge_root(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(workbench_read::check_workbench_knowledge_root(&path))
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

/// Return WORKBENCH_KNOWLEDGE repos (derived from repo-list) with `local_exists`.
/// KNOWLEDGE_CORPUS is deprecated for list UI — use `get_sediment_kb_repos` instead.
pub fn kb_corpus_status_json(filter_type: Option<&str>) -> Result<Value, String> {
    let filter = filter_type.unwrap_or("KNOWLEDGE_CORPUS");
    if filter == "KNOWLEDGE_CORPUS" {
        return Ok(json!({
            "deprecated": true,
            "repos": [],
        }));
    }

    let repo_root = repo_root().map_err(|e| format!("{e:?}"))?;
    let workbench_knowledge_root =
        crate::config::meili_env::workbench_knowledge_root_path(&repo_root);

    let cache_path = crate::config::paths::repo_list_cache_path().map_err(|e| format!("{e:?}"))?;
    if !cache_path.is_file() {
        return Ok(json!({ "repos": [] }));
    }
    let text = std::fs::read_to_string(&cache_path).map_err(|e| e.to_string())?;
    let data: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let repos: Vec<Value> = data
        .get("repos")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|repo| {
                    if repo.get("type").and_then(|v| v.as_str()) != Some(filter) {
                        return None;
                    }
                    let full_name = repo.get("full_name")?.as_str()?;
                    let name = full_name.split('/').next_back().unwrap_or(full_name);
                    let local_exists = workbench_knowledge_root
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|dir_name| dir_name == name)
                        .unwrap_or(false)
                        && workbench_knowledge_root.join(".git").exists();
                    Some(json!({
                        "full_name": full_name,
                        "name": name,
                        "description": repo.get("description").cloned().unwrap_or(Value::Null),
                        "local_exists": local_exists,
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(json!({ "repos": repos }))
}

/// Return every WORKBENCH_KNOWLEDGE repo (derived from repo-list) together with
/// a `local_exists` flag indicating whether the repo is already cloned locally.
/// KNOWLEDGE_CORPUS list UI must use sediment-kb API; this command returns deprecated empty data.
#[tauri::command]
pub fn get_kb_corpus_status(
    _app: AppHandle,
    filter_type: Option<String>,
) -> Result<Value, String> {
    kb_corpus_status_json(filter_type.as_deref())
}

#[tauri::command]
pub fn get_kb_diff_status(_app: AppHandle) -> Result<Value, String> {
    let repo_root = repo_root()?;
    let knowledge_corpus_root = std::path::PathBuf::from(
        crate::config::meili_env::knowledge_corpus_root_string(&repo_root),
    );

    let repos: Vec<Value> = workbench_read::get_topics(&repo_root)
        .get("topics")
        .and_then(|topics| topics.as_array())
        .map(|topics| {
            topics
                .iter()
                .filter_map(|item| {
                    let full_name = item.get("repo")?.as_str()?;
                    let name = full_name.split('/').next_back().unwrap_or(full_name);
                    if !knowledge_corpus_root.join(name).is_dir() {
                        return None;
                    }

                    let status = crate::services::kb::kb_status_json(&repo_root, full_name);
                    let has_changes = status
                        .get("total")
                        .and_then(|total| total.as_u64())
                        .map(|total| total > 0)
                        .unwrap_or(false);

                    Some(json!({
                        "full_name": full_name,
                        "has_changes": has_changes,
                    }))
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(json!({ "repos": repos }))
}

pub fn sediment_kb_categories_json() -> Result<Value, String> {
    sediment_kb::ensure_uncategorized().map_err(|e| e.to_string())?;
    let cats = sediment_kb::load_categories().map_err(|e| e.to_string())?;
    Ok(json!({ "categories": cats.categories }))
}

pub fn sediment_kb_repos_json() -> Result<Value, String> {
    sediment_kb::ensure_uncategorized().map_err(|e| e.to_string())?;
    let categories = sediment_kb::load_categories().map_err(|e| e.to_string())?;
    let repos = sediment_kb::load_repos().map_err(|e| e.to_string())?;
    let name_by_id: std::collections::HashMap<&str, &str> = categories
        .categories
        .iter()
        .map(|c| (c.id.as_str(), c.name.as_str()))
        .collect();
    let enriched: Vec<Value> = repos
        .repos
        .iter()
        .map(|r| {
            json!({
                "full_name": r.full_name,
                "description": r.description,
                "category_id": r.category_id,
                "category_name": name_by_id
                    .get(r.category_id.as_str())
                    .copied()
                    .unwrap_or("未分类"),
            })
        })
        .collect();
    Ok(json!({ "repos": enriched }))
}

#[tauri::command]
pub fn get_sediment_kb_categories(_app: AppHandle) -> Result<Value, String> {
    sediment_kb_categories_json()
}

#[tauri::command]
pub fn get_sediment_kb_repos(_app: AppHandle) -> Result<Value, String> {
    sediment_kb_repos_json()
}

#[cfg(test)]
#[path = "../unit-tests/commands/sediment_kb_read.rs"]
mod sediment_kb_read_tests;

#[cfg(test)]
#[path = "../unit-tests/commands/kb_corpus_status.rs"]
mod kb_corpus_status_tests;
