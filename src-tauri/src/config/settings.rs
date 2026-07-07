//! User settings in `~/.config/lulu-workbench/config.toml` (override via `LULU_WB_CONFIG_DIR` in tests).

use std::fs;
use std::path::{Path, PathBuf};
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

pub const DEFAULT_GITHUB_USER_URL: &str = "";

/// Runtime branch for prod vs automated test sandbox vs manual cache-first debug.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestModeKind {
    Prod,
    TestSandbox,
    ManualCacheFirst,
}

/// True when process env `TEST_MODE` equals `"1"`.
pub fn is_test_mode() -> bool {
    std::env::var("TEST_MODE").ok().as_deref() == Some("1")
}

/// Three-state branch: prod / automated test sandbox / manual cache-first debug.
///
/// Both automated tests and manual debugging use `TEST_MODE=1`; `cfg(test)` distinguishes
/// the automated sandbox from manual cache-first overlay at runtime.
pub fn test_mode_kind() -> TestModeKind {
    if !is_test_mode() {
        return TestModeKind::Prod;
    }
    #[cfg(test)]
    {
        return TestModeKind::TestSandbox;
    }
    #[cfg(not(test))]
    {
        return TestModeKind::ManualCacheFirst;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_workbench_knowledge_root")]
    pub workbench_knowledge_root: PathBuf,
    #[serde(default = "default_knowledge_corpus_root")]
    pub knowledge_corpus_root: PathBuf,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    #[serde(default = "default_meili_url")]
    pub meili_url: String,
    #[serde(default = "default_github_user_url")]
    pub github_user_url: String,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn default_workbench_knowledge_root() -> PathBuf {
    home_dir().join("Code")
}

fn default_knowledge_corpus_root() -> PathBuf {
    home_dir().join("Code")
}

pub fn default_cache_dir() -> PathBuf {
    home_dir().join(".cache").join("lulu-workbench")
}

fn default_meili_url() -> String {
    "http://localhost:7700".to_string()
}

fn default_github_user_url() -> String {
    DEFAULT_GITHUB_USER_URL.to_string()
}

/// Personal GitHub home (`https://github.com/{owner}`) + workbench clone dir name → blob base for file links.
pub fn workbench_github_blob_base(github_user_url: &str, workbench_knowledge_root: &Path) -> String {
    let trimmed = github_user_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    let repo = workbench_knowledge_root
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("lulu-workbench-knowledge");
    format!("{trimmed}/{repo}/blob/main")
}

/// `https://github.com/{owner}` from `git remote get-url origin` when workbench root is a git repo.
pub fn infer_github_user_url_from_workbench_root(workbench_root: &Path) -> Option<String> {
    if !workbench_root.is_dir() || !workbench_root.join(".git").exists() {
        return None;
    }
    let out = crate::integrations::git::exec(workbench_root, &["remote", "get-url", "origin"]).ok()?;
    if !out.success {
        return None;
    }
    github_user_home_from_remote_url(out.stdout.trim())
}

/// Parse owner home URL from a GitHub remote (HTTPS or SSH).
pub fn github_user_home_from_remote_url(remote: &str) -> Option<String> {
    let s = remote.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix("https://github.com/").or_else(|| s.strip_prefix("http://github.com/")) {
        let owner = rest.split('/').next()?.trim();
        if owner.is_empty() {
            return None;
        }
        return Some(format!("https://github.com/{owner}"));
    }
    if let Some(rest) = s.strip_prefix("git@github.com:") {
        let owner = rest.split('/').next()?.trim();
        if owner.is_empty() {
            return None;
        }
        return Some(format!("https://github.com/{owner}"));
    }
    if let Some(rest) = s.strip_prefix("ssh://git@github.com/") {
        let owner = rest.split('/').next()?.trim();
        if owner.is_empty() {
            return None;
        }
        return Some(format!("https://github.com/{owner}"));
    }
    None
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            workbench_knowledge_root: default_workbench_knowledge_root(),
            knowledge_corpus_root: default_knowledge_corpus_root(),
            cache_dir: default_cache_dir(),
            meili_url: default_meili_url(),
            github_user_url: default_github_user_url(),
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

/// Config directory: test override → `$LULU_WB_CONFIG_DIR` (tests only) → `~/.config/lulu-workbench`.
pub fn settings_config_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(lock) = TEST_CONFIG_DIR.get() {
        if let Some(ref p) = *lock.lock().expect("test config lock") {
            return p.clone();
        }
    }
    #[cfg(test)]
    if let Ok(dir) = std::env::var("LULU_WB_CONFIG_DIR") {
        return PathBuf::from(dir);
    }
    home_dir().join(".config").join("lulu-workbench")
}

/// True when `cache_dir` points at OS/tempfile ephemeral storage (must not persist in user config).
pub(crate) fn is_unstable_cache_dir(path: &Path) -> bool {
    let s = path.to_string_lossy();
    if s.starts_with("/tmp/") || s == "/tmp" {
        return true;
    }
    // macOS `$TMPDIR`: `/var/folders/.../T/.tmpXXXXXX/...`
    if s.contains("/T/.tmp") {
        return true;
    }
    for comp in path.components() {
        if let std::path::Component::Normal(name) = comp {
            let n = name.to_string_lossy();
            if n.starts_with(".tmp") && n.len() > 4 {
                return true;
            }
        }
    }
    false
}

fn skip_cache_dir_normalization() -> bool {
    #[cfg(test)]
    if let Some(lock) = TEST_CONFIG_DIR.get() {
        if lock.lock().expect("test config lock").is_some() {
            return true;
        }
    }
    false
}

/// Reset poisoned `cache_dir` values (e.g. test temp paths written via `set_config`).
pub(crate) fn normalize_cache_dir(settings: &mut AppSettings) {
    if skip_cache_dir_normalization() {
        return;
    }
    if is_unstable_cache_dir(&settings.cache_dir) {
        settings.cache_dir = default_cache_dir();
    }
}

#[cfg(test)]
pub fn write_test_config(
    dir: &Path,
    workbench_knowledge_root: &Path,
    knowledge_corpus_root: Option<&Path>,
) {
    let kb = knowledge_corpus_root
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| home_dir().join("Code").display().to_string());
    fs::write(
        dir.join("config.toml"),
        format!(
            "workbench_knowledge_root = \"{}\"\nknowledge_corpus_root = \"{}\"\n",
            workbench_knowledge_root.display(),
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
    let mut settings: AppSettings = toml::from_str(&text)?;
    normalize_cache_dir(&mut settings);
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
        "workbench_knowledge_root": settings.workbench_knowledge_root.to_string_lossy(),
        "knowledge_corpus_root": settings.knowledge_corpus_root.to_string_lossy(),
        "github_user_url": settings.github_user_url,
        "meili_url": settings.meili_url,
        "cache_dir": settings.cache_dir.to_string_lossy(),
        "has_github_token": has_github_token,
        "has_meili_key": has_meili_key,
    })
}

/// Apply `set_config` payload keys onto settings (toml fields only).
pub fn apply_config_payload(settings: &mut AppSettings, payload: &serde_json::Value) {
    if let Some(v) = payload
        .get("workbench_knowledge_root")
        .and_then(|x| x.as_str())
    {
        settings.workbench_knowledge_root = PathBuf::from(v);
    }
    if let Some(v) = payload
        .get("knowledge_corpus_root")
        .and_then(|x| x.as_str())
    {
        settings.knowledge_corpus_root = PathBuf::from(v);
    }
    if let Some(v) = payload.get("github_user_url").and_then(|x| x.as_str()) {
        settings.github_user_url = v.to_string();
    }
    if let Some(v) = payload.get("meili_url").and_then(|x| x.as_str()) {
        settings.meili_url = v.to_string();
    }
    // `cache_dir` is not user-settable via API; use `default_cache_dir()` / manual toml edit.
}

#[cfg(test)]
#[path = "../unit-tests/config/settings.rs"]
mod tests;
