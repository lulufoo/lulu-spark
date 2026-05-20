use serde_json::Value;
use tauri::AppHandle;

use crate::config::paths;
use crate::integrations::{gh_read, meilisearch};
use crate::services::knowledge::knowledge_index_json;
use crate::services::workbench_read;

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub fn get_knowledge_index(_app: AppHandle, force: Option<bool>) -> Result<Value, String> {
    let _ = force;
    let path = paths::knowledge_index_path().map_err(|e| format!("{e:?}"))?;
    Ok(knowledge_index_json(&path))
}

#[tauri::command]
pub fn search_knowledge(_app: AppHandle, q: String, limit: Option<u32>) -> Result<Value, String> {
    let root = repo_root()?;
    Ok(meilisearch::search_json(&root, "knowledge", &q, limit))
}

#[tauri::command]
pub fn search_workbench(_app: AppHandle, q: String, limit: Option<u32>) -> Result<Value, String> {
    let root = repo_root()?;
    Ok(meilisearch::search_json(&root, "workbench", &q, limit))
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
pub fn get_status(_app: AppHandle) -> Result<Value, String> {
    Ok(workbench_read::get_status(&repo_root()?))
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

#[tauri::command]
pub fn get_repo_list(_app: AppHandle, force: Option<bool>) -> Result<Value, String> {
    Ok(gh_read::repo_list_json(
        &repo_root()?,
        force.unwrap_or(false),
    ))
}

#[tauri::command]
pub fn get_repo_dirs(_app: AppHandle, repo: String) -> Result<Value, String> {
    Ok(gh_read::repo_dirs_json(&repo_root()?, &repo))
}

#[tauri::command]
pub fn check_file(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    Ok(gh_read::check_file_json(&repo_root()?, &repo, &path))
}
