//! User settings in `~/.config/lulu-workbench/config.toml` (prod) or `dev.config.toml` (TEST_MODE / tests).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const DEV_CONFIG_FILE_NAME: &str = "dev.config.toml";
pub const PROD_CONFIG_FILE_NAME: &str = "config.toml";

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LlmSettings {
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
}

/// One typed LLM settings entry in `AppSettings.llm` (`host` | `cursor`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmSettingsEntry {
    #[serde(rename = "type")]
    pub engine_type: String,
    #[serde(default)]
    pub platform: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
}

impl LlmSettingsEntry {
    fn fields(&self) -> LlmSettings {
        LlmSettings {
            platform: self.platform.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
        }
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
    /// Assistant engine selection: `host` | `cursor`. Default `host`.
    /// Illegal values are rejected at route resolve time (not silently remapped).
    #[serde(default = "default_assistant_engine")]
    pub assistant_engine: String,
    /// Per-engine LLM settings list (`[[llm]]` in toml). Entries may be absent.
    /// T5_FOLLOW_ON_MIGRATION: legacy single-slot→list is an out-of-band local script; main code must not dual-read.
    #[serde(default)]
    pub llm: Vec<LlmSettingsEntry>,
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

fn default_assistant_engine() -> String {
    "host".to_string()
}

/// Result of mapping legacy LLM settings into the Engine storage slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineSettingsSlice {
    pub assistant_engine: String,
    pub model: String,
    pub platform: String,
    pub base_url: String,
    pub host_api_key: Option<String>,
}

fn normalize_engine_value(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "host" => Some("host"),
        "cursor" => Some("cursor"),
        _ => None,
    }
}

/// Resolve the list entry for a legal engine type (`host` | `cursor`).
/// Missing type or empty list → `None` (no panic). Illegal type → `None`.
pub fn llm_entry_by_type<'a>(
    entries: &'a [LlmSettingsEntry],
    engine_type: &str,
) -> Option<&'a LlmSettingsEntry> {
    let normalized = normalize_engine_value(engine_type)?;
    entries
        .iter()
        .find(|e| normalize_engine_value(&e.engine_type) == Some(normalized))
}

/// Update or insert a typed LLM entry. Illegal/unknown type is rejected (not written).
pub fn upsert_llm_entry(
    entries: &mut Vec<LlmSettingsEntry>,
    engine_type: &str,
    fields: &LlmSettings,
) -> Result<(), SettingsError> {
    let Some(normalized) = normalize_engine_value(engine_type) else {
        return Err(SettingsError::ConfigGuard(format!(
            "invalid llm entry type: {engine_type}"
        )));
    };
    if let Some(existing) = entries
        .iter_mut()
        .find(|e| normalize_engine_value(&e.engine_type) == Some(normalized))
    {
        existing.engine_type = normalized.to_string();
        existing.platform = fields.platform.clone();
        existing.base_url = fields.base_url.clone();
        existing.model = fields.model.clone();
    } else {
        entries.push(LlmSettingsEntry {
            engine_type: normalized.to_string(),
            platform: fields.platform.clone(),
            base_url: fields.base_url.clone(),
            model: fields.model.clone(),
        });
    }
    Ok(())
}

/// Built-in readonly preset metadata (aligned with frontend `engine-presets.js`).
/// Model remains independently editable; platform/base_url are never client-writable.
fn builtin_preset_fields(engine: &str) -> Option<(&'static str, &'static str)> {
    match normalize_engine_value(engine).unwrap_or("host") {
        "cursor" => Some(("cursor_agent", "(managed by Cursor Agent)")),
        // OpenAI-compatible path (docs.bigmodel.cn); chat_url appends /chat/completions when base ends in /v4.
        "host" => Some(("glm", "https://open.bigmodel.cn/api/paas/v4")),
        _ => None,
    }
}

/// Stamp readonly preset fields from the current Engine category.
/// Ensures Host `load_llm_config` receives a usable base_url matching the UI preset.
pub fn stamp_readonly_preset_fields(settings: &mut AppSettings) {
    let Some(engine) = normalize_engine_value(&settings.assistant_engine) else {
        return;
    };
    if let Some((platform, base_url)) = builtin_preset_fields(engine) {
        let mut fields = llm_entry_by_type(&settings.llm, engine)
            .map(LlmSettingsEntry::fields)
            .unwrap_or_default();
        fields.platform = platform.to_string();
        fields.base_url = base_url.to_string();
        let _ = upsert_llm_entry(&mut settings.llm, engine, &fields);
    }
}

/// Map legacy `LlmSettings` (+ optional api key) into Engine classification + Model + metadata.
///
/// - No / blank / illegal `existing_engine` → lock classification to `host`
/// - Legal `host`|`cursor` → respect it; still carry Credential/Model from legacy
/// - `platform` / `base_url` become readonly preset metadata on the slice
pub fn migrate_llm_to_engine(
    legacy: &LlmSettings,
    api_key: Option<&str>,
    existing_engine: Option<&str>,
) -> EngineSettingsSlice {
    let assistant_engine = existing_engine
        .and_then(normalize_engine_value)
        .unwrap_or("host")
        .to_string();
    let host_api_key = api_key
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    EngineSettingsSlice {
        assistant_engine,
        model: legacy.model.clone(),
        platform: legacy.platform.clone(),
        base_url: legacy.base_url.clone(),
        host_api_key,
    }
}

fn apply_engine_migration_on_load(settings: &mut AppSettings) {
    let existing = settings.assistant_engine.trim();
    let existing = (!existing.is_empty()).then_some(existing);
    let lookup = existing.unwrap_or("host");
    let current_fields = llm_entry_by_type(&settings.llm, lookup)
        .map(LlmSettingsEntry::fields)
        .unwrap_or_default();
    let slice = migrate_llm_to_engine(&current_fields, None, existing);
    settings.assistant_engine = slice.assistant_engine;
    // Preserve non-empty platform/base_url on the current entry. Stamp builtin preset
    // only when the entry is missing or both fields are blank.
    let needs_stamp = match llm_entry_by_type(&settings.llm, &settings.assistant_engine) {
        Some(e) => e.platform.trim().is_empty() && e.base_url.trim().is_empty(),
        None => true,
    };
    if needs_stamp {
        stamp_readonly_preset_fields(settings);
    }
    let _ = crate::config::secrets::migrate_legacy_llm_api_key_to_host();
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
            assistant_engine: default_assistant_engine(),
            llm: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum SettingsError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Serialize(toml::ser::Error),
    ConfigGuard(String),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SettingsError::Io(e) => write!(f, "io: {e}"),
            SettingsError::Parse(e) => write!(f, "parse: {e}"),
            SettingsError::Serialize(e) => write!(f, "serialize: {e}"),
            SettingsError::ConfigGuard(msg) => write!(f, "config guard: {msg}"),
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

/// Isolated config dir for script/subprocess tests (`LULU_WB_CONFIG_DIR`), not dev.config.toml.
#[cfg(test)]
fn isolated_config_dir() -> Option<PathBuf> {
    std::env::var("LULU_WB_CONFIG_DIR")
        .ok()
        .map(PathBuf::from)
}

/// Config directory: `$LULU_WB_CONFIG_DIR` (isolated tests) → `~/.config/lulu-workbench`.
pub fn settings_config_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(dir) = isolated_config_dir() {
        return dir;
    }
    home_dir().join(".config").join("lulu-workbench")
}

/// True when loading/saving `dev.config.toml` instead of prod `config.toml`.
pub fn uses_dev_config() -> bool {
    #[cfg(test)]
    if isolated_config_dir().is_some() {
        return false;
    }
    if is_test_mode() {
        return true;
    }
    #[cfg(test)]
    {
        return true;
    }
    #[cfg(not(test))]
    false
}

pub fn prod_config_file_path() -> PathBuf {
    settings_config_dir().join(PROD_CONFIG_FILE_NAME)
}

pub fn dev_config_file_path() -> PathBuf {
    settings_config_dir().join(DEV_CONFIG_FILE_NAME)
}

pub fn config_file_path() -> PathBuf {
    if uses_dev_config() {
        dev_config_file_path()
    } else {
        prod_config_file_path()
    }
}

/// Read prod `config.toml` only (for TestSandbox prod-path guards).
pub fn load_prod_settings() -> AppSettings {
    let path = prod_config_file_path();
    if !path.is_file() {
        return AppSettings::default();
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return AppSettings::default();
    };
    let Ok(mut settings) = toml::from_str::<AppSettings>(&text) else {
        return AppSettings::default();
    };
    normalize_prod_paths(&mut settings);
    settings
}

/// True when path points at OS/tempfile ephemeral storage (must not persist in prod config).
pub(crate) fn is_unstable_path(path: &Path) -> bool {
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

pub(crate) fn is_unstable_cache_dir(path: &Path) -> bool {
    is_unstable_path(path)
}

fn config_is_prod_file(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()) == Some(PROD_CONFIG_FILE_NAME)
}

fn should_normalize_for_path(path: &Path) -> bool {
    #[cfg(test)]
    if isolated_config_dir().is_some() {
        return false;
    }
    config_is_prod_file(path)
}

pub(crate) fn normalize_prod_paths(settings: &mut AppSettings) {
    if is_unstable_cache_dir(&settings.cache_dir) {
        settings.cache_dir = default_cache_dir();
    }
    if is_unstable_path(&settings.workbench_knowledge_root) {
        settings.workbench_knowledge_root = default_workbench_knowledge_root();
    }
    if is_unstable_path(&settings.knowledge_corpus_root) {
        settings.knowledge_corpus_root = default_knowledge_corpus_root();
    }
}

/// Reset poisoned `cache_dir` values (e.g. test temp paths written via `set_config`).
pub(crate) fn normalize_cache_dir(settings: &mut AppSettings) {
    if is_unstable_cache_dir(&settings.cache_dir) {
        settings.cache_dir = default_cache_dir();
    }
}

fn guard_save_path(path: &Path) -> Result<(), SettingsError> {
    let file = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if uses_dev_config() && file == PROD_CONFIG_FILE_NAME {
        return Err(SettingsError::ConfigGuard(
            "tests/TEST_MODE must not write config.toml; use dev.config.toml".into(),
        ));
    }
    if !uses_dev_config() && file == DEV_CONFIG_FILE_NAME {
        return Err(SettingsError::ConfigGuard(
            "prod runtime must not write dev.config.toml".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
pub fn write_test_config_with_cache(
    _dir: &Path,
    workbench_knowledge_root: &Path,
    knowledge_corpus_root: Option<&Path>,
    cache_dir: Option<&Path>,
) {
    let corpus = knowledge_corpus_root
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| home_dir().join("Code").display().to_string());
    let cache = cache_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| _dir.join("cache"));
    fs::create_dir_all(&cache).expect("mkdir cache");
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir config dir");
    }
    fs::write(
        &path,
        format!(
            "workbench_knowledge_root = \"{}\"\nknowledge_corpus_root = \"{}\"\ncache_dir = \"{}\"\n",
            workbench_knowledge_root.display(),
            corpus,
            cache.display()
        ),
    )
    .expect("write test config");
}

pub fn load() -> Result<AppSettings, SettingsError> {
    let path = config_file_path();
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(&path)?;
    let mut settings: AppSettings = toml::from_str(&text)?;
    if should_normalize_for_path(&path) {
        normalize_prod_paths(&mut settings);
    }
    apply_engine_migration_on_load(&mut settings);
    Ok(settings)
}

pub fn save(settings: &AppSettings) -> Result<(), SettingsError> {
    let path = config_file_path();
    guard_save_path(&path)?;
    let mut to_save = settings.clone();
    if should_normalize_for_path(&path) {
        normalize_prod_paths(&mut to_save);
    }
    let dir = settings_config_dir();
    fs::create_dir_all(&dir)?;
    let text = toml::to_string_pretty(&to_save)?;
    fs::write(path, text)?;
    Ok(())
}

/// JSON shape for `get_config` / `set_config` (frontend field names).
/// Never echoes plaintext api keys — only `has_*_key` hints.
pub fn to_config_json(
    settings: &AppSettings,
    has_github_token: bool,
    has_meili_key: bool,
    has_host_key: bool,
    has_cursor_key: bool,
) -> serde_json::Value {
    let current = llm_entry_by_type(&settings.llm, &settings.assistant_engine)
        .map(LlmSettingsEntry::fields)
        .unwrap_or_default();
    serde_json::json!({
        "workbench_knowledge_root": settings.workbench_knowledge_root.to_string_lossy(),
        "knowledge_corpus_root": settings.knowledge_corpus_root.to_string_lossy(),
        "github_user_url": settings.github_user_url,
        "meili_url": settings.meili_url,
        "cache_dir": settings.cache_dir.to_string_lossy(),
        "assistant_engine": settings.assistant_engine,
        "has_github_token": has_github_token,
        "has_meili_key": has_meili_key,
        "has_llm_key": has_host_key,
        "has_host_key": has_host_key,
        "has_cursor_key": has_cursor_key,
        "llm": {
            "platform": current.platform,
            "base_url": current.base_url,
            "model": current.model,
        },
    })
}

/// Apply `set_config` payload keys onto settings (toml fields only).
/// Illegal `assistant_engine` values are rejected (aligned with `resolve_engine`: host|cursor).
/// Flat `llm` maps to the current `assistant_engine` list entry (create if missing).
/// `llm.platform` / `llm.base_url` from the client are ignored (preset readonly); when
/// `assistant_engine` or `llm.model` is applied they are stamped from the builtin category preset.
pub fn apply_config_payload(
    settings: &mut AppSettings,
    payload: &serde_json::Value,
) -> Result<(), SettingsError> {
    let mut engine_touched = false;
    let mut llm_model_touched = false;
    if let Some(v) = payload.get("assistant_engine").and_then(|x| x.as_str()) {
        let Some(normalized) = normalize_engine_value(v) else {
            return Err(SettingsError::ConfigGuard(format!(
                "invalid assistant_engine value: {v}"
            )));
        };
        settings.assistant_engine = normalized.to_string();
        engine_touched = true;
    }
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
    if let Some(llm) = payload.get("llm").and_then(|x| x.as_object()) {
        // Preset fields are readonly — ignore client platform/base_url.
        if let Some(v) = llm.get("model").and_then(|x| x.as_str()) {
            let engine =
                normalize_engine_value(&settings.assistant_engine).unwrap_or("host");
            let mut fields = llm_entry_by_type(&settings.llm, engine)
                .map(LlmSettingsEntry::fields)
                .unwrap_or_default();
            fields.model = v.to_string();
            upsert_llm_entry(&mut settings.llm, engine, &fields)?;
            llm_model_touched = true;
        }
    }
    if engine_touched || llm_model_touched {
        stamp_readonly_preset_fields(settings);
    }
    // `cache_dir` is not user-settable via API; use `default_cache_dir()` / manual toml edit.
    Ok(())
}

#[cfg(test)]
#[path = "../unit-tests/config/settings.rs"]
mod tests;
