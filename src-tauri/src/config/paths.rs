use std::path::PathBuf;

use crate::config::settings::{self, AppSettings, SettingsError};

#[derive(Debug)]
pub enum PathsError {
    Settings(SettingsError),
    RepoRootUnavailable,
    InvalidMasterTaskId,
}

impl From<SettingsError> for PathsError {
    fn from(e: SettingsError) -> Self {
        PathsError::Settings(e)
    }
}

pub fn repo_root() -> Result<PathBuf, PathsError> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or(PathsError::RepoRootUnavailable)
}

fn settings() -> Result<AppSettings, PathsError> {
    Ok(settings::load()?)
}

pub fn cache_dir() -> Result<PathBuf, PathsError> {
    Ok(settings()?.cache_dir)
}

pub fn spark_root() -> Result<PathBuf, PathsError> {
    Ok(settings()?.spark_root)
}

/// Durable user store: `{runtime_cache_dir}/data`.
/// Prod: `{HOME}/.cache/lulu-spark/data`; sandbox: `<sandbox_dir>/cache/data`.
/// Does not follow `cache_dir` or `spark_root` in config.
pub fn runtime_data_dir() -> Result<PathBuf, PathsError> {
    Ok(settings::runtime_cache_dir()?.join("data"))
}

/// Durable metadata database: `{runtime_data_dir}/spark-data.sqlite`.
pub fn spark_data_db_path() -> Result<PathBuf, PathsError> {
    Ok(runtime_data_dir()?.join("spark-data.sqlite"))
}

/// One folder per workspace: `{runtime_data_dir}/workspaces/{workspace_id}/`.
pub fn workspaces_root() -> Result<PathBuf, PathsError> {
    Ok(runtime_data_dir()?.join("workspaces"))
}

pub fn read_later_path() -> Result<PathBuf, PathsError> {
    Ok(runtime_data_dir()?.join("read_later").join("read_later.json"))
}

/// Runtime unread ledger: `{runtime_cache_dir}/message_center/message_center.json`.
/// Does not follow `cache_dir` or `spark_root` in config.
pub fn message_center_path() -> Result<PathBuf, PathsError> {
    Ok(settings::runtime_cache_dir()?
        .join("message_center")
        .join("message_center.json"))
}

/// Per-channel MCP tool checkboxes: `{spark_root}/mcp_channel_tools.json`.
pub fn mcp_channel_tools_path() -> Result<PathBuf, PathsError> {
    Ok(spark_root()?.join("mcp_channel_tools.json"))
}

pub fn todo_tasks_path() -> Result<PathBuf, PathsError> {
    Ok(spark_root()?
        .join("todo_tasks")
        .join("todo_tasks.json"))
}

pub fn todo_tasks_dir() -> Result<PathBuf, PathsError> {
    Ok(spark_root()?.join("todo_tasks"))
}

pub fn todo_tasks_index_path() -> Result<PathBuf, PathsError> {
    Ok(todo_tasks_dir()?.join("index.json"))
}

pub fn todo_tasks_categories_path() -> Result<PathBuf, PathsError> {
    Ok(todo_tasks_dir()?.join("categories.json"))
}

fn validate_master_task_id(master_task_id: &str) -> Result<&str, PathsError> {
    if master_task_id.trim().is_empty() {
        return Err(PathsError::InvalidMasterTaskId);
    }
    Ok(master_task_id)
}

pub fn todo_tasks_task_dir(master_task_id: &str) -> Result<PathBuf, PathsError> {
    let master_task_id = validate_master_task_id(master_task_id)?;
    Ok(todo_tasks_dir()?.join("tasks").join(master_task_id))
}

pub fn todo_tasks_sub_tasks_path(master_task_id: &str) -> Result<PathBuf, PathsError> {
    Ok(todo_tasks_task_dir(master_task_id)?.join("sub_tasks.json"))
}

pub fn todo_tasks_plan_md_path(master_task_id: &str) -> Result<PathBuf, PathsError> {
    Ok(todo_tasks_task_dir(master_task_id)?.join("todo.md"))
}

pub fn cache_todo_tasks_v1_path() -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?.join("todo_tasks.json"))
}

/// Hidden files rules of one workspace: `{cache_dir}/workspace-hide-patterns/{workspace_id}.json`.
/// The caller validates `workspace_id`.
pub fn workspace_hide_patterns_path(workspace_id: &str) -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?
        .join("workspace-hide-patterns")
        .join(format!("{workspace_id}.json")))
}

/// The single global rule list from before rules were per workspace:
/// `{cache_dir}/knowledge-hide-patterns.json`. Read only, to seed a workspace's first rules.
pub fn legacy_hide_patterns_path() -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?.join("knowledge-hide-patterns.json"))
}

/// The document a workspace's page had open: `{cache_dir}/workspace-viewer-state/{workspace_id}.json`.
/// The caller validates `workspace_id`.
pub fn workspace_viewer_state_path(workspace_id: &str) -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?
        .join("workspace-viewer-state")
        .join(format!("{workspace_id}.json")))
}

/// Local highlight cache root: `{cache_dir}/doc-highlights`.
pub fn doc_highlights_dir() -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?.join("doc-highlights"))
}

/// `{cache_dir}/doc-highlights/{md5}.json` — `md5` is 32 lowercase hex chars.
pub fn doc_highlights_file(md5_hex: &str) -> Result<PathBuf, PathsError> {
    let md5_hex = md5_hex.trim();
    if md5_hex.len() != 32 || !md5_hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(PathsError::RepoRootUnavailable);
    }
    let dir = doc_highlights_dir()?;
    let target = dir.join(format!("{md5_hex}.json"));
    if !target.starts_with(&dir) {
        return Err(PathsError::RepoRootUnavailable);
    }
    Ok(target)
}

pub fn draft_path(common_path: &str) -> Result<PathBuf, PathsError> {
    let drafts_dir = cache_dir()?.join("drafts");
    let mut target = drafts_dir.clone();
    for comp in std::path::Path::new(common_path).components() {
        match comp {
            std::path::Component::Normal(s) => target.push(s),
            _ => return Err(PathsError::RepoRootUnavailable),
        }
    }
    if !target.starts_with(&drafts_dir) {
        return Err(PathsError::RepoRootUnavailable);
    }
    Ok(target)
}

/// Crash-buffer path for note create: `{cache_dir}/drafts/docs/<temp_id>`.
pub fn doc_draft_path(temp_id: &str) -> Result<PathBuf, PathsError> {
    if temp_id.trim().is_empty() {
        return Err(PathsError::RepoRootUnavailable);
    }
    let notes_dir = cache_dir()?.join("drafts").join("notes");
    let mut target = notes_dir.clone();
    for comp in std::path::Path::new(temp_id).components() {
        match comp {
            std::path::Component::Normal(s) => target.push(s),
            _ => return Err(PathsError::RepoRootUnavailable),
        }
    }
    if !target.starts_with(&notes_dir) || target == notes_dir {
        return Err(PathsError::RepoRootUnavailable);
    }
    Ok(target)
}

#[cfg(test)]
#[path = "../unit-tests/config/paths.rs"]
mod tests;
