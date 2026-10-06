use serde_json::{json, Value};
use tauri::AppHandle;

use crate::config::paths;
use crate::config::roots::notes_root_path;
use crate::services::keyword_index::{
    cache_dir_or_err, search_desktop_knowledge, search_desktop_spark,
};
use crate::services::sediment_kb;
use crate::services::tags_registry;
use crate::services::spark_read;

fn repo_root() -> Result<std::path::PathBuf, String> {
    paths::repo_root().map_err(|e| format!("{e:?}"))
}

#[tauri::command]
pub async fn search_knowledge(
    _app: AppHandle,
    q: String,
    limit: Option<u32>,
) -> Result<Value, String> {
    let _ = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || {
        let cache = cache_dir_or_err()?;
        Ok(search_desktop_knowledge(&cache, &q, limit))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_spark(
    _app: AppHandle,
    q: String,
    limit: Option<u32>,
) -> Result<Value, String> {
    let _ = repo_root()?;
    tauri::async_runtime::spawn_blocking(move || {
        let cache = cache_dir_or_err()?;
        Ok(search_desktop_spark(&cache, &q, limit))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_topics(_app: AppHandle) -> Result<Value, String> {
    Ok(spark_read::get_topics(&repo_root()?))
}

#[tauri::command]
pub fn get_annotations(_app: AppHandle) -> Result<Value, String> {
    Ok(spark_read::get_annotations_summary(&repo_root()?))
}

#[tauri::command]
pub fn get_tags_registry(_app: AppHandle) -> Result<Value, String> {
    let notes = notes_root_path(&repo_root()?);
    Ok(tags_registry::read_registry(&notes))
}

#[tauri::command]
pub fn get_annotation(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(spark_read::get_annotation(&repo_root()?, &path))
}

#[tauri::command]
pub fn get_draft(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(spark_read::get_draft(&repo_root()?, &path))
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
    Ok(spark_read::get_config(&repo_root()?))
}

#[tauri::command]
pub fn kb_read(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    Ok(crate::services::knowledge::kb_read_json(&repo_root()?, &repo, &path))
}

#[tauri::command]
pub fn get_kb_asset(
    _app: AppHandle,
    repo: String,
    base: String,
    href: String,
) -> Result<Value, String> {
    Ok(crate::services::knowledge::kb_asset_json(
        &repo_root()?,
        &repo,
        &base,
        &href,
    ))
}

#[tauri::command]
pub fn kb_list(
    _app: AppHandle,
    repo: String,
    path: String,
    mode: Option<String>,
) -> Result<Value, String> {
    let mode = mode.unwrap_or_else(|| "flat".into());
    Ok(crate::services::knowledge::kb_list_json(
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
    category_id: Option<String>,
) -> Result<Value, String> {
    Ok(crate::services::knowledge::kb_doc_count_json(
        &repo_root()?,
        &repo,
        category_id.as_deref(),
    ))
}

#[tauri::command]
pub fn get_kb_hide_patterns(_app: AppHandle) -> Result<Value, String> {
    crate::services::knowledge::kb_hide_patterns_json()
}

#[tauri::command]
pub fn kb_annotation(_app: AppHandle, repo: String, path: String) -> Result<Value, String> {
    Ok(crate::services::knowledge::kb_annotation_json(&repo_root()?, &repo, &path))
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
pub fn get_notes_index(_app: AppHandle) -> Result<Value, String> {
    Ok(spark_read::get_notes_index(&repo_root()?))
}

#[tauri::command]
pub fn get_notes_file(
    _app: AppHandle,
    layer: String,
    path: String,
) -> Result<Value, String> {
    Ok(spark_read::get_notes_file(&repo_root()?, &layer, &path))
}

#[tauri::command]
pub fn read_abs_file(_app: AppHandle, path: String) -> Result<Value, String> {
    Ok(crate::services::abs_file::read_abs_file(&path))
}

#[tauri::command]
pub fn get_notes_asset(
    _app: AppHandle,
    layer: String,
    base: String,
    href: String,
) -> Result<Value, String> {
    Ok(spark_read::get_notes_asset(
        &repo_root()?,
        &layer,
        &base,
        &href,
    ))
}

pub fn sediment_kb_categories_json() -> Result<Value, String> {
    sediment_kb::ensure_uncategorized().map_err(|e| e.to_string())?;
    let cats = sediment_kb::load_categories().map_err(|e| e.to_string())?;
    Ok(json!({ "categories": cats.categories }))
}

fn sediment_kb_local_exists(repo_root: &std::path::Path, full_name: &str) -> bool {
    let knowledge_root = std::path::PathBuf::from(
        crate::config::roots::knowledge_root_string(repo_root),
    );
    let name = full_name.split('/').next_back().unwrap_or(full_name);
    knowledge_root.join(name).is_dir()
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
                    .unwrap_or(crate::services::sediment_kb::UNCATEGORIZED_NAME),
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

pub fn get_message_channel_unread_json(channel: &str) -> Result<bool, String> {
    Ok(crate::services::message_center::channel_unread(channel))
}

#[tauri::command]
pub fn get_message_channel_unread(_app: AppHandle, channel: String) -> Result<bool, String> {
    get_message_channel_unread_json(&channel)
}

#[cfg(test)]
#[path = "../unit-tests/commands/kb_doc_count.rs"]
mod kb_doc_count_tests;

#[cfg(test)]
#[path = "../unit-tests/commands/sediment_kb_read.rs"]
mod sediment_kb_read_tests;

#[cfg(test)]
#[path = "../unit-tests/commands/message_center.rs"]
mod message_center_tests;
