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
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_config_dir<F: FnOnce(&std::path::Path)>(f: F) {
        let _g = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock");
        let dir = tempfile::tempdir().expect("tmp");
        settings::set_test_config_dir(Some(dir.path().to_path_buf()));
        f(dir.path());
        settings::set_test_config_dir(None);
    }

    #[test]
    fn cache_dir_uses_settings_not_repo_dot_cache() {
        with_config_dir(|cfg| {
            let custom = cfg.join("custom-cache");
            fs::write(
                cfg.join("config.toml"),
                format!(r#"cache_dir = "{}""#, custom.display()),
            )
            .expect("write");
            let got = cache_dir().expect("cache_dir");
            assert_eq!(got, custom);
            let root = repo_root().expect("repo");
            assert_ne!(got, root.join(".cache"));
        });
    }

    #[test]
    fn repo_root_matches_cargo_manifest_parent() {
        let root = repo_root().expect("repo_root");
        let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("parent")
            .to_path_buf();
        assert_eq!(root, expected);
    }
}
