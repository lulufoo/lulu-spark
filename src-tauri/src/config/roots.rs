//! Spark root helper.

use std::path::{Path, PathBuf};

use crate::config::settings::{self, AppSettings};

fn settings_or_default() -> AppSettings {
    settings::load().unwrap_or_default()
}

pub fn spark_root_path(_repo_root: &Path) -> PathBuf {
    settings_or_default().spark_root
}
