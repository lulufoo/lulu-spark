//! Spark / notes / knowledge root helpers.

use std::path::{Path, PathBuf};

use crate::config::settings::{self, AppSettings};

fn settings_or_default() -> AppSettings {
    settings::load().unwrap_or_default()
}

pub fn spark_root_path(_repo_root: &Path) -> PathBuf {
    settings_or_default().spark_root
}

/// Notes files: `{HOME}/.cache/lulu-spark/data/notes`.
pub fn notes_root_path(_repo_root: &Path) -> PathBuf {
    crate::config::paths::runtime_data_dir().join("notes")
}

pub fn knowledge_root_string(_repo_root: &Path) -> String {
    crate::config::paths::runtime_knowledge_dir()
        .to_string_lossy()
        .into_owned()
}

pub fn github_user_url_string(_repo_root: &Path) -> String {
    settings_or_default().github_user_url
}
