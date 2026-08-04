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

pub fn workbench_knowledge_root() -> Result<PathBuf, PathsError> {
    Ok(settings()?.workbench_knowledge_root)
}

pub fn knowledge_corpus_root() -> Result<PathBuf, PathsError> {
    Ok(settings()?.knowledge_corpus_root)
}

pub fn sediment_kb_dir() -> Result<PathBuf, PathsError> {
    Ok(workbench_knowledge_root()?.join("sediment-kb"))
}

pub fn read_later_path() -> Result<PathBuf, PathsError> {
    Ok(workbench_knowledge_root()?
        .join("read_later")
        .join("read_later.json"))
}

pub fn plan_tasks_path() -> Result<PathBuf, PathsError> {
    Ok(workbench_knowledge_root()?
        .join("todo_tasks")
        .join("todo_tasks.json"))
}

pub fn plan_tasks_dir() -> Result<PathBuf, PathsError> {
    Ok(workbench_knowledge_root()?.join("todo_tasks"))
}

pub fn plan_tasks_index_path() -> Result<PathBuf, PathsError> {
    Ok(plan_tasks_dir()?.join("index.json"))
}

pub fn plan_tasks_categories_path() -> Result<PathBuf, PathsError> {
    Ok(plan_tasks_dir()?.join("categories.json"))
}

fn validate_master_task_id(master_task_id: &str) -> Result<&str, PathsError> {
    if master_task_id.trim().is_empty() {
        return Err(PathsError::InvalidMasterTaskId);
    }
    Ok(master_task_id)
}

pub fn plan_tasks_task_dir(master_task_id: &str) -> Result<PathBuf, PathsError> {
    let master_task_id = validate_master_task_id(master_task_id)?;
    Ok(plan_tasks_dir()?.join("tasks").join(master_task_id))
}

pub fn plan_tasks_sub_tasks_path(master_task_id: &str) -> Result<PathBuf, PathsError> {
    Ok(plan_tasks_task_dir(master_task_id)?.join("sub_tasks.json"))
}

pub fn plan_tasks_plan_md_path(master_task_id: &str) -> Result<PathBuf, PathsError> {
    Ok(plan_tasks_task_dir(master_task_id)?.join("todo.md"))
}

pub fn cache_plan_tasks_v1_path() -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?.join("todo_tasks.json"))
}

pub fn sediment_kb_categories_path() -> Result<PathBuf, PathsError> {
    Ok(sediment_kb_dir()?.join("categories.json"))
}

pub fn sediment_kb_repos_path() -> Result<PathBuf, PathsError> {
    Ok(sediment_kb_dir()?.join("repos.json"))
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

/// Crash-buffer path for note create: `{cache_dir}/drafts/notes/<temp_id>`.
pub fn notes_draft_path(temp_id: &str) -> Result<PathBuf, PathsError> {
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
