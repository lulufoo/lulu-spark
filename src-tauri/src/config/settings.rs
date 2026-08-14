//! User settings in `config.toml` under the active config root
//! (`~/.config/lulu-workbench/` or sandbox dirs selected by `TestSandbox` env).

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::ffi::OsString;
#[cfg(test)]
use std::io::Write;
#[cfg(test)]
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub const PROD_CONFIG_FILE_NAME: &str = "config.toml";

pub const DEFAULT_GITHUB_USER_URL: &str = "";
pub const HOST_LLM_PLATFORM: &str = "glm";
pub const HOST_LLM_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

pub const DEFAULT_PROD_HTTP_PORT: u16 = 8765;
pub const DEFAULT_PROD_MCP_PORT: u16 = 9876;
pub const DEFAULT_SANDBOX_HTTP_PORT: u16 = 18765;
pub const DEFAULT_SANDBOX_MCP_PORT: u16 = 19876;

const ENV_TEST_SANDBOX: &str = "TestSandbox";
const ENV_TEST_SANDBOX_ID: &str = "TestSandboxId";

#[cfg(test)]
static TEST_MACHINE_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Record the machine config path before a test fixture changes `HOME`.
#[cfg(test)]
pub(crate) fn register_test_machine_config_path(path: PathBuf) {
    if let Some(registered) = TEST_MACHINE_CONFIG_PATH.get() {
        assert_eq!(
            registered, &path,
            "test machine config path changed during one test process"
        );
        return;
    }
    if let Err(registered) = TEST_MACHINE_CONFIG_PATH.set(path.clone()) {
        assert_eq!(
            registered, path,
            "test machine config path changed during one test process"
        );
    }
}

/// True when env `TestSandbox` is `true` or `1`.
pub fn is_test_sandbox() -> bool {
    matches!(
        std::env::var(ENV_TEST_SANDBOX).ok().as_deref(),
        Some("true") | Some("1")
    )
}

/// Optional instance id; `None` if unset/empty. Does not validate charset.
pub fn test_sandbox_id_raw() -> Option<String> {
    std::env::var(ENV_TEST_SANDBOX_ID)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Validate `TestSandboxId`: `[A-Za-z0-9_-]{1,64}`.
pub fn validate_test_sandbox_id(id: &str) -> Result<(), SettingsError> {
    if id.is_empty() || id.len() > 64 {
        return Err(SettingsError::ConfigGuard(format!(
            "invalid TestSandboxId length: {id:?} (need 1..=64)"
        )));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(SettingsError::ConfigGuard(format!(
            "invalid TestSandboxId charset: {id:?} (allowed [A-Za-z0-9_-])"
        )));
    }
    Ok(())
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

/// One typed LLM settings entry in `AppSettings.llm` (`host`).
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

/// True when an entry selects the only supported Host platform.
pub fn is_supported_host_llm_entry(entry: &LlmSettingsEntry) -> bool {
    normalize_engine_value(&entry.engine_type) == Some("host")
        && entry.platform.trim().eq_ignore_ascii_case(HOST_LLM_PLATFORM)
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
    /// Assistant engine selection: `host` (Agent Loop + GLM). Default `host`.
    /// Legacy or empty values are retained on disk but treated as unconfigured.
    #[serde(default = "default_assistant_engine")]
    pub assistant_engine: String,
    /// Per-engine LLM settings list (`[[llm]]` in toml). Entries may be absent.
    /// T5_FOLLOW_ON_MIGRATION: legacy single-slot→list is an out-of-band local script; main code must not dual-read.
    #[serde(default)]
    pub llm: Vec<LlmSettingsEntry>,
    /// Sidecar HTTP listen port. `None` → plane default (prod 8765 / sandbox 18765).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_port: Option<u16>,
    /// Host MCP listen port. `None` → plane default (prod 9876 / sandbox 19876).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_port: Option<u16>,
}

impl AppSettings {
    pub fn effective_http_port(&self) -> u16 {
        self.http_port.unwrap_or(if is_test_sandbox() {
            DEFAULT_SANDBOX_HTTP_PORT
        } else {
            DEFAULT_PROD_HTTP_PORT
        })
    }

    pub fn effective_mcp_port(&self) -> u16 {
        self.mcp_port.unwrap_or(if is_test_sandbox() {
            DEFAULT_SANDBOX_MCP_PORT
        } else {
            DEFAULT_PROD_MCP_PORT
        })
    }
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

fn normalize_engine_value(raw: &str) -> Option<&'static str> {
    (raw.trim().eq_ignore_ascii_case("host")).then_some("host")
}

/// Resolve the list entry for the supported engine type (`host`).
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
    match normalize_engine_value(engine) {
        // OpenAI-compatible path (docs.bigmodel.cn); chat_url appends /chat/completions when base ends in /v4.
        Some("host") => Some((HOST_LLM_PLATFORM, HOST_LLM_BASE_URL)),
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

fn stamp_host_preset_on_load(settings: &mut AppSettings) {
    let is_host = normalize_engine_value(&settings.assistant_engine).is_some();
    let needs_stamp = match llm_entry_by_type(&settings.llm, "host") {
        Some(entry) => entry.platform.trim().is_empty() && entry.base_url.trim().is_empty(),
        None => true,
    };
    if is_host && needs_stamp {
        stamp_readonly_preset_fields(settings);
    }
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
            http_port: None,
            mcp_port: None,
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

/// Fixed prod config directory (never follows `TestSandbox`).
pub fn prod_config_dir() -> PathBuf {
    home_dir().join(".config").join("lulu-workbench")
}

/// Shared sandbox config directory (secrets + template `config.toml`).
pub fn shared_sandbox_config_dir() -> PathBuf {
    home_dir().join(".config").join("lulu-workbench-sandbox")
}

/// Active config directory from `TestSandbox` / `TestSandboxId`.
///
/// Invalid `TestSandboxId` returns `Err` — callers must fail closed.
pub fn settings_config_dir() -> Result<PathBuf, SettingsError> {
    if !is_test_sandbox() {
        return Ok(prod_config_dir());
    }
    match test_sandbox_id_raw() {
        None => Ok(shared_sandbox_config_dir()),
        Some(id) => {
            validate_test_sandbox_id(&id)?;
            Ok(home_dir()
                .join(".config")
                .join(format!("lulu-workbench-sandbox-{id}")))
        }
    }
}

pub fn prod_config_file_path() -> PathBuf {
    prod_config_dir().join(PROD_CONFIG_FILE_NAME)
}

pub fn config_file_path() -> Result<PathBuf, SettingsError> {
    Ok(settings_config_dir()?.join(PROD_CONFIG_FILE_NAME))
}

/// Read prod `config.toml` only (for sandbox prod-path guards).
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
    // Temp HOME fixtures (unit tests) keep explicit roots; do not heal to ~/Code.
    if !path.ancestors().any(is_unstable_path) {
        normalize_prod_paths(&mut settings);
    }
    settings
}

fn path_equals_or_under(path: &Path, root: &Path) -> bool {
    if path == root {
        return true;
    }
    path.starts_with(root)
}

/// Fail-closed sandbox guard: roots / meili / ports must not collide with prod.
pub fn validate_sandbox_against_prod(
    sandbox: &AppSettings,
    prod: &AppSettings,
) -> Result<(), SettingsError> {
    if !is_test_sandbox() {
        return Ok(());
    }
    for (label, path, prod_root) in [
        (
            "workbench_knowledge_root",
            sandbox.workbench_knowledge_root.as_path(),
            prod.workbench_knowledge_root.as_path(),
        ),
        (
            "knowledge_corpus_root",
            sandbox.knowledge_corpus_root.as_path(),
            prod.knowledge_corpus_root.as_path(),
        ),
        (
            "cache_dir",
            sandbox.cache_dir.as_path(),
            prod.cache_dir.as_path(),
        ),
    ] {
        if path.as_os_str().is_empty() {
            return Err(SettingsError::ConfigGuard(format!(
                "sandbox required field empty: {label}"
            )));
        }
        if path_equals_or_under(path, prod_root) {
            return Err(SettingsError::ConfigGuard(format!(
                "sandbox {label} collides with prod root {}",
                prod_root.display()
            )));
        }
    }
    if sandbox.meili_url.trim().is_empty() {
        return Err(SettingsError::ConfigGuard(
            "sandbox required field empty: meili_url".into(),
        ));
    }
    if sandbox.meili_url == prod.meili_url {
        return Err(SettingsError::ConfigGuard(
            "sandbox meili_url collides with prod".into(),
        ));
    }
    let sh = sandbox.effective_http_port();
    let sm = sandbox.effective_mcp_port();
    // Prod ports must not use sandbox plane defaults (env may be TestSandbox=true here).
    let ph = prod.http_port.unwrap_or(DEFAULT_PROD_HTTP_PORT);
    let pm = prod.mcp_port.unwrap_or(DEFAULT_PROD_MCP_PORT);
    if sh == ph || sm == pm || sh == pm || sm == ph {
        return Err(SettingsError::ConfigGuard(format!(
            "sandbox ports collide with prod (sandbox http={sh} mcp={sm}, prod http={ph} mcp={pm})"
        )));
    }
    Ok(())
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
    // Sandbox / temp HOME fixtures must keep written roots (do not "heal" to ~/Code).
    if is_test_sandbox() {
        return false;
    }
    if path.ancestors().any(is_unstable_path) {
        return false;
    }
    path == prod_config_file_path() && config_is_prod_file(path)
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

#[cfg(test)]
enum GuardPathComponent {
    Normal(OsString),
    Parent,
}

#[cfg(test)]
fn canonical_path_or_existing_parent(path: &Path) -> Result<PathBuf, SettingsError> {
    match fs::canonicalize(path) {
        Ok(resolved) => return Ok(resolved),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(SettingsError::ConfigGuard(format!(
                "cannot resolve config path {}: {error}",
                path.display()
            )));
        }
    }

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(SettingsError::Io)?
            .join(path)
    };
    let mut probe = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        match fs::canonicalize(probe) {
            Ok(mut resolved) => {
                for component in suffix.iter().rev() {
                    match component {
                        GuardPathComponent::Normal(name) => resolved.push(name),
                        GuardPathComponent::Parent => {
                            if !resolved.pop() {
                                return Err(SettingsError::ConfigGuard(format!(
                                    "config path escapes filesystem root: {}",
                                    path.display()
                                )));
                            }
                        }
                    }
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(SettingsError::ConfigGuard(format!(
                    "cannot resolve config path {}: {error}",
                    path.display()
                )));
            }
        }
        let component = probe.components().next_back().ok_or_else(|| {
            SettingsError::ConfigGuard(format!("cannot resolve config path {}", path.display()))
        })?;
        match component {
            std::path::Component::Normal(name) => {
                suffix.push(GuardPathComponent::Normal(name.to_os_string()));
            }
            std::path::Component::ParentDir => suffix.push(GuardPathComponent::Parent),
            std::path::Component::CurDir => {}
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(SettingsError::ConfigGuard(format!(
                    "cannot resolve config path {}",
                    path.display()
                )));
            }
        }
        probe = probe.parent().ok_or_else(|| {
            SettingsError::ConfigGuard(format!("cannot resolve config path {}", path.display()))
        })?;
    }
}

/// Reject an alias that resolves to a protected formal config path.
#[cfg(test)]
pub(crate) fn reject_write_to_protected_config(
    path: &Path,
    protected_config_path: &Path,
) -> Result<(), SettingsError> {
    let resolved_path = canonical_path_or_existing_parent(path)?;
    let resolved_protected = canonical_path_or_existing_parent(protected_config_path)?;
    if resolved_path == resolved_protected {
        return Err(SettingsError::ConfigGuard(format!(
            "test config write targets protected machine config: {}",
            path.display()
        )));
    }
    Ok(())
}

/// Reject test writes until the machine formal config path is registered.
#[cfg(test)]
pub(crate) fn reject_test_write_to_machine_config(path: &Path) -> Result<(), SettingsError> {
    let protected = TEST_MACHINE_CONFIG_PATH.get().ok_or_else(|| {
        SettingsError::ConfigGuard(
            "machine formal config path was not registered before test write".into(),
        )
    })?;
    reject_write_to_protected_config(path, protected)
}

/// Atomically replace an already-verified test config target.
#[cfg(test)]
pub(crate) fn atomic_write_test_config(
    verified_target_path: &Path,
    content: &str,
) -> Result<(), SettingsError> {
    let parent = verified_target_path.parent().ok_or_else(|| {
        SettingsError::ConfigGuard(format!(
            "test config path has no parent: {}",
            verified_target_path.display()
        ))
    })?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(content.as_bytes())?;
    temporary.flush()?;
    temporary
        .persist(verified_target_path)
        .map_err(|error| SettingsError::Io(error.error))?;
    Ok(())
}

#[cfg(test)]
pub fn write_test_config_with_cache(
    config_path: &Path,
    workbench_knowledge_root: &Path,
    knowledge_corpus_root: Option<&Path>,
    cache_dir: Option<&Path>,
    meili_url: &str,
    http_port: u16,
    mcp_port: u16,
) -> Result<(), SettingsError> {
    let corpus = knowledge_corpus_root
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| workbench_knowledge_root.display().to_string());
    let cache = cache_dir
        .map(|p| p.to_path_buf())
        .or_else(|| config_path.parent().map(|parent| parent.join("cache")))
        .ok_or_else(|| {
            SettingsError::ConfigGuard(format!(
                "test config path has no parent: {}",
                config_path.display()
            ))
        })?;
    reject_test_write_to_machine_config(config_path)?;
    fs::create_dir_all(&cache)?;
    let text = format!(
        "workbench_knowledge_root = \"{}\"\nknowledge_corpus_root = \"{}\"\ncache_dir = \"{}\"\nmeili_url = \"{meili_url}\"\nhttp_port = {http_port}\nmcp_port = {mcp_port}\n",
        workbench_knowledge_root.display(),
        corpus,
        cache.display()
    );
    atomic_write_test_config(config_path, &text)
}

pub fn load() -> Result<AppSettings, SettingsError> {
    let path = config_file_path()?;
    if is_test_sandbox() && !path.is_file() {
        return Err(SettingsError::ConfigGuard(format!(
            "sandbox config missing (initiator must fork): {}",
            path.display()
        )));
    }
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let text = fs::read_to_string(&path)?;
    let mut settings: AppSettings = toml::from_str(&text)?;
    if should_normalize_for_path(&path) {
        normalize_prod_paths(&mut settings);
    }
    stamp_host_preset_on_load(&mut settings);
    if is_test_sandbox() {
        validate_sandbox_against_prod(&settings, &load_prod_settings())?;
    }
    Ok(settings)
}

pub fn save(settings: &AppSettings) -> Result<(), SettingsError> {
    let path = config_file_path()?;
    let mut to_save = settings.clone();
    if should_normalize_for_path(&path) {
        normalize_prod_paths(&mut to_save);
    }
    if is_test_sandbox() {
        validate_sandbox_against_prod(&to_save, &load_prod_settings())?;
    }
    let text = toml::to_string_pretty(&to_save)?;

    #[cfg(test)]
    {
        reject_test_write_to_machine_config(&path)?;
        atomic_write_test_config(&path, &text)
    }

    #[cfg(not(test))]
    {
        let dir = settings_config_dir()?;
        fs::create_dir_all(&dir)?;
        fs::write(path, text)?;
        Ok(())
    }
}

/// JSON shape for `get_config` / `set_config` (frontend field names).
/// Never echoes plaintext api keys — only the Host/GLM key hint.
pub fn to_config_json(
    settings: &AppSettings,
    has_github_token: bool,
    has_meili_key: bool,
    has_host_key: bool,
) -> serde_json::Value {
    let current = if normalize_engine_value(&settings.assistant_engine).is_some() {
        llm_entry_by_type(&settings.llm, "host")
            .map(LlmSettingsEntry::fields)
            .unwrap_or_default()
    } else {
        LlmSettings::default()
    };
    serde_json::json!({
        "workbench_knowledge_root": settings.workbench_knowledge_root.to_string_lossy(),
        "knowledge_corpus_root": settings.knowledge_corpus_root.to_string_lossy(),
        "github_user_url": settings.github_user_url,
        "meili_url": settings.meili_url,
        "cache_dir": settings.cache_dir.to_string_lossy(),
        "assistant_engine": settings.assistant_engine,
        "http_port": settings.effective_http_port(),
        "mcp_port": settings.effective_mcp_port(),
        "test_sandbox": is_test_sandbox(),
        "has_github_token": has_github_token,
        "has_meili_key": has_meili_key,
        "has_host_key": has_host_key,
        "llm": {
            "platform": current.platform,
            "base_url": current.base_url,
            "model": current.model,
        },
    })
}

/// Apply `set_config` payload keys onto settings (toml fields only).
/// Illegal or legacy `assistant_engine` values are rejected.
/// Flat `llm` maps to the Host list entry (create if missing).
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
