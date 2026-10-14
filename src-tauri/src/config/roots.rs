//! Spark / notes / knowledge root helpers.
//!
//! Notes and knowledge roots read the process runtime plane installed at boot
//! (or by `TestConfigEnv`). They never fall back to prod cache paths.

use std::path::{Path, PathBuf};

use crate::config::settings::{self, AppSettings};

fn settings_or_default() -> AppSettings {
    settings::load().unwrap_or_default()
}

pub fn spark_root_path(_repo_root: &Path) -> PathBuf {
    settings_or_default().spark_root
}

/// Notes files: `{runtime_cache_dir}/data/notes`.
pub fn notes_root_path(_repo_root: &Path) -> PathBuf {
    settings::runtime_cache_dir()
        .expect("runtime plane not initialized; boot or TestConfigEnv must install it")
        .join("data")
        .join("notes")
}

/// Knowledge clones: `{runtime_cache_dir}/data/knowledge`.
pub fn knowledge_root_string(_repo_root: &Path) -> String {
    settings::runtime_cache_dir()
        .expect("runtime plane not initialized; boot or TestConfigEnv must install it")
        .join("data")
        .join("knowledge")
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
#[path = "../unit-tests/config/roots.rs"]
mod tests;
