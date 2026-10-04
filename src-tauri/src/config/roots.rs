//! Spark / notes / knowledge root helpers from `config.toml`.

use std::path::{Path, PathBuf};

use crate::config::settings::{self, AppSettings};

fn settings_or_default() -> AppSettings {
    settings::load().unwrap_or_default()
}

pub fn spark_root_path(_repo_root: &Path) -> PathBuf {
    settings_or_default().spark_root
}

/// Notes files: `{spark_root}/notes`.
pub fn notes_root_path(_repo_root: &Path) -> PathBuf {
    spark_root_path(_repo_root).join("notes")
}

pub fn knowledge_root_string(_repo_root: &Path) -> String {
    settings_or_default()
        .knowledge_root
        .to_string_lossy()
        .into_owned()
}

pub fn github_user_url_string(_repo_root: &Path) -> String {
    settings_or_default().github_user_url
}
