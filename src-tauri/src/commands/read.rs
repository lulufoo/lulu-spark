use serde_json::{json, Value};
use tauri::AppHandle;

use crate::config::paths;
use crate::config::meili_env::workbench_knowledge_root_path;
use crate::integrations::{gh_read, meilisearch};
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
pub fn get_note_draft(_app: AppHandle, temp_id: String) -> Result<Value, String> {
    Ok(crate::services::draft::load_note_draft_json(&temp_id))
}

#[tauri::command]
pub fn get_doc_highlights(_app: AppHandle, key: String) -> Result<Value, String> {
    Ok(crate::services::doc_highlights::get_doc_highlights(&key))
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
pub fn kb_list(
    _app: AppHandle,
    repo: String,
    path: String,
    mode: Option<String>,
) -> Result<Value, String> {
    let mode = mode.unwrap_or_else(|| "flat".into());
    Ok(crate::services::kb::kb_list_json(
        &repo_root()?,
        &repo,
        &path,
        &mode,
    ))
}

#[tauri::command]
pub fn kb_doc_count(
    _app: AppHandle,
    repo: String,
    hide_pattern: Option<String>,
    category_id: Option<String>,
) -> Result<Value, String> {
    Ok(crate::services::kb::kb_doc_count_json(
        &repo_root()?,
        &repo,
        hide_pattern.as_deref(),
        category_id.as_deref(),
    ))
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

#[tauri::command]
pub fn get_corpus_asset(
    _app: AppHandle,
    layer: String,
    base: String,
    href: String,
) -> Result<Value, String> {
    Ok(workbench_read::get_corpus_asset(
        &repo_root()?,
        &layer,
        &base,
        &href,
    ))
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

fn sediment_kb_local_exists(repo_root: &std::path::Path, full_name: &str) -> bool {
    let knowledge_corpus_root = std::path::PathBuf::from(
        crate::config::meili_env::knowledge_corpus_root_string(repo_root),
    );
    let name = full_name.split('/').next_back().unwrap_or(full_name);
    knowledge_corpus_root.join(name).is_dir()
}

pub fn sediment_kb_repos_json(repo_root: &std::path::Path) -> Result<Value, String> {
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
                "local_exists": sediment_kb_local_exists(repo_root, &r.full_name),
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
    sediment_kb_repos_json(&repo_root()?)
}

#[cfg(test)]
#[path = "../unit-tests/commands/kb_doc_count.rs"]
mod kb_doc_count_tests;

#[cfg(test)]
#[path = "../unit-tests/commands/sediment_kb_read.rs"]
mod sediment_kb_read_tests;
