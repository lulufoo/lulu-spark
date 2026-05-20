//! Legacy module name — runtime values come from `config.toml` + Keychain (`settings` / `secrets`).
//! Does **not** read `repo_root/meili.env`.

use std::path::{Path, PathBuf};

use crate::config::secrets::{self, KEY_MEILI_MASTER};
use crate::config::settings::{self, AppSettings};

fn settings_or_default() -> AppSettings {
    settings::load().unwrap_or_default()
}

pub fn workbench_knowledge_root_path(_repo_root: &Path) -> PathBuf {
    settings_or_default().workbench_knowledge_root
}

pub fn knowledge_corpus_root_string(_repo_root: &Path) -> String {
    settings_or_default()
        .knowledge_corpus_root
        .to_string_lossy()
        .into_owned()
}

pub fn meili_url(_repo_root: &Path) -> String {
    settings_or_default().meili_url
}

pub fn meili_master_key(_repo_root: &Path) -> String {
    let _ = _repo_root;
    secrets::get_secret(KEY_MEILI_MASTER)
        .ok()
        .flatten()
        .unwrap_or_else(|| "lulu-workbench-local".to_string())
}

pub fn github_user_url_string(_repo_root: &Path) -> String {
    settings_or_default().github_user_url
}
