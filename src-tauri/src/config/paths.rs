use std::path::PathBuf;

use crate::config::settings::{self, AppSettings, SettingsError};

#[derive(Debug)]
pub enum PathsError {
    Settings(SettingsError),
    RepoRootUnavailable,
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

pub fn repo_list_cache_path() -> Result<PathBuf, PathsError> {
    Ok(cache_dir()?.join("repo-list.json"))
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

#[cfg(test)]
#[path = "../unit-tests/config/paths.rs"]
mod tests;
