//! User settings in `~/.config/lulu-workbench/config.toml` (override via `LULU_WB_CONFIG_DIR` in tests).

use std::fs;
use std::path::PathBuf;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

#[cfg(test)]
static TEST_CONFIG_DIR: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

#[cfg(test)]
pub fn set_test_config_dir(dir: Option<PathBuf>) {
    let lock = TEST_CONFIG_DIR.get_or_init(|| Mutex::new(None));
    *lock.lock().expect("test config lock") = dir;
}

pub const DEFAULT_CORPUS_GITHUB: &str =
    "https://github.com/lulufoo/lulu-workbench-knowledge/blob/main";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_corpus_root")]
    pub corpus_root: PathBuf,
    #[serde(default = "default_knowledge_base_dir")]
    pub knowledge_base_dir: PathBuf,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    #[serde(default = "default_meili_url")]
    pub meili_url: String,
    #[serde(default = "default_knowledge_corpus_github")]
    pub knowledge_corpus_github: String,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn default_corpus_root() -> PathBuf {
    home_dir().join("Code")
}

fn default_knowledge_base_dir() -> PathBuf {
    home_dir().join("Code")
}

pub fn default_cache_dir() -> PathBuf {
    home_dir().join(".cache").join("lulu-workbench")
}

fn default_meili_url() -> String {
    "http://localhost:7700".to_string()
}

fn default_knowledge_corpus_github() -> String {
    DEFAULT_CORPUS_GITHUB.to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            corpus_root: default_corpus_root(),
            knowledge_base_dir: default_knowledge_base_dir(),
            cache_dir: default_cache_dir(),
            meili_url: default_meili_url(),
            knowledge_corpus_github: default_knowledge_corpus_github(),
        }
    }
}

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsError::Io(e) => write!(f, "io: {e}"),
            SettingsError::Parse(e) => write!(f, "parse: {e}"),
            SettingsError::Serialize(e) => write!(f, "serialize: {e}"),
        }
    }
}

impl From<std::io::Error> for SettingsError {
    fn from(e: std::io::Error) -> Self {
        SettingsError::Io(e)
    }
}

impl From<toml::de::Error> for SettingsError {
    fn from(e: toml::de::Error) -> Self {
        SettingsError::Parse(e)
    }
}

impl From<toml::ser::Error> for SettingsError {
    fn from(e: toml::ser::Error) -> Self {
        SettingsError::Serialize(e)
    }
}

/// Config directory: test override → `$LULU_WB_CONFIG_DIR` → `~/.config/lulu-workbench`.
pub fn settings_config_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(lock) = TEST_CONFIG_DIR.get() {
        if let Some(ref p) = *lock.lock().expect("test config lock") {
            return p.clone();
        }
    }
    if let Ok(dir) = std::env::var("LULU_WB_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    home_dir().join(".config").join("lulu-workbench")
}

#[cfg(test)]
pub fn write_test_config(dir: &Path, corpus_root: &Path, knowledge_base_dir: Option<&Path>) {
    let kb = knowledge_base_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| home_dir().join("Code").display().to_string());
    fs::write(
        dir.join("config.toml"),
        format!(
            "corpus_root = \"{}\"\nknowledge_base_dir = \"{}\"\n",
            corpus_root.display(),
            kb
        ),
    )
    .expect("write test config");
    set_test_config_dir(Some(dir.to_path_buf()));
}

pub fn config_file_path() -> PathBuf {
    settings_config_dir().join("config.toml")
}

pub fn load() -> Result<AppSettings, SettingsError> {
    let path = config_file_path();
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(&path)?;
    let settings: AppSettings = toml::from_str(&text)?;
    Ok(settings)
}

pub fn save(settings: &AppSettings) -> Result<(), SettingsError> {
    let dir = settings_config_dir();
    fs::create_dir_all(&dir)?;
    let text = toml::to_string_pretty(settings)?;
    fs::write(config_file_path(), text)?;
    Ok(())
}

/// JSON shape for `get_config` / `set_config` (frontend field names).
pub fn to_config_json(
    settings: &AppSettings,
    has_github_token: bool,
    has_meili_key: bool,
) -> serde_json::Value {
    serde_json::json!({
        "archive_root": settings.corpus_root.to_string_lossy(),
        "kb_root": settings.knowledge_base_dir.to_string_lossy(),
        "corpus_github": settings.knowledge_corpus_github,
        "meili_url": settings.meili_url,
        "cache_dir": settings.cache_dir.to_string_lossy(),
        "has_github_token": has_github_token,
        "has_meili_key": has_meili_key,
    })
}

/// Apply `set_config` payload keys onto settings (toml fields only).
pub fn apply_config_payload(settings: &mut AppSettings, payload: &serde_json::Value) {
    if let Some(v) = payload.get("archive_root").and_then(|x| x.as_str()) {
        settings.corpus_root = PathBuf::from(v);
    }
    if let Some(v) = payload.get("kb_root").and_then(|x| x.as_str()) {
        settings.knowledge_base_dir = PathBuf::from(v);
    }
    if let Some(v) = payload.get("corpus_github").and_then(|x| x.as_str()) {
        settings.knowledge_corpus_github = v.to_string();
    }
    if let Some(v) = payload.get("meili_url").and_then(|x| x.as_str()) {
        settings.meili_url = v.to_string();
    }
    if let Some(v) = payload.get("cache_dir").and_then(|x| x.as_str()) {
        settings.cache_dir = PathBuf::from(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard;

    impl EnvGuard {
        fn with_config_dir(dir: &Path) -> Self {
            let _g = env_lock().lock().expect("lock");
            set_test_config_dir(Some(dir.to_path_buf()));
            crate::config::secrets::test_secrets_clear();
            Self
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            let _g = env_lock().lock().expect("lock");
            set_test_config_dir(None);
            crate::config::secrets::test_secrets_clear();
        }
    }

    #[test]
    fn load_reads_config_toml_fields() {
        let dir = tempfile::tempdir().expect("tmp");
        let _guard = EnvGuard::with_config_dir(dir.path());
        let corpus = dir.path().join("my-corpus");
        let kb = dir.path().join("my-kb");
        let cache = dir.path().join("my-cache");
        fs::write(
            dir.path().join("config.toml"),
            format!(
                r#"
corpus_root = "{}"
knowledge_base_dir = "{}"
cache_dir = "{}"
meili_url = "http://127.0.0.1:7701"
knowledge_corpus_github = "https://example.com/repo"
"#,
                corpus.display(),
                kb.display(),
                cache.display()
            ),
        )
        .expect("write");
        let s = load().expect("load");
        assert_eq!(s.corpus_root, corpus);
        assert_eq!(s.knowledge_base_dir, kb);
        assert_eq!(s.cache_dir, cache);
        assert_eq!(s.meili_url, "http://127.0.0.1:7701");
        assert_eq!(s.knowledge_corpus_github, "https://example.com/repo");
    }

    #[test]
    fn load_defaults_when_no_config_file() {
        let dir = tempfile::tempdir().expect("tmp");
        let _guard = EnvGuard::with_config_dir(dir.path());
        let s = load().expect("load");
        assert_eq!(s.cache_dir, default_cache_dir());
        assert_eq!(s.meili_url, "http://localhost:7700");
    }

    #[test]
    fn save_roundtrip_updates_file() {
        let dir = tempfile::tempdir().expect("tmp");
        let _guard = EnvGuard::with_config_dir(dir.path());
        let mut s = AppSettings::default();
        s.corpus_root = dir.path().join("corpus-x");
        save(&s).expect("save");
        let s2 = load().expect("reload");
        assert_eq!(s2.corpus_root, s.corpus_root);
    }

    #[test]
    fn to_config_json_includes_flags_without_token() {
        let s = AppSettings::default();
        let v = to_config_json(&s, true, false);
        assert_eq!(v["has_github_token"], true);
        assert_eq!(v["has_meili_key"], false);
        assert!(v.get("github_token").is_none());
        assert!(v.get("meili_master_key").is_none());
    }
}
