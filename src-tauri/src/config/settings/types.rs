use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const PROD_CONFIG_FILE_NAME: &str = "config.toml";

pub const DEFAULT_GITHUB_USER_URL: &str = "";
pub const HOST_LLM_PLATFORM: &str = "glm";
pub const HOST_LLM_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

pub const DEFAULT_PROD_HTTP_PORT: u16 = 8765;
pub const DEFAULT_PROD_MCP_PORT: u16 = 9876;
pub const DEFAULT_PROD_GATEWAY_PORT: u16 = 7654;
pub const DEFAULT_SANDBOX_HTTP_PORT: u16 = 18765;
pub const DEFAULT_SANDBOX_MCP_PORT: u16 = 19876;
pub const DEFAULT_SANDBOX_GATEWAY_PORT: u16 = 17654;

pub(super) const ENV_TEST_SANDBOX: &str = "TestSandbox";
pub(super) const ENV_TEST_SANDBOX_ID: &str = "TestSandboxId";

/// True when env `TestSandbox` is `true` or `1`.
pub fn is_test_sandbox() -> bool {
    matches!(
        std::env::var(ENV_TEST_SANDBOX).ok().as_deref(),
        Some("true") | Some("1")
    )
}

/// Keychain-backed stores stay in process memory.
/// `TestSandbox` is the product switch. `cfg(test)` keeps `cargo test --lib`
/// off the login keychain even when a case did not open a sandbox.
pub fn uses_in_memory_keychain() -> bool {
    cfg!(test) || is_test_sandbox()
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
    pub(super) fn fields(&self) -> LlmSettings {
        LlmSettings {
            platform: self.platform.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_workbench_root")]
    pub workbench_root: PathBuf,
    #[serde(default = "default_knowledge_root")]
    pub knowledge_root: PathBuf,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    #[serde(default = "default_github_user_url")]
    pub github_user_url: String,
    /// Optional Workbench GitHub repository URL (`https://github.com/owner/repo`). Empty = none.
    #[serde(default)]
    pub workbench_github_repo_url: String,
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
    /// LAN HTTPS Gateway listen port. `None` → plane default (prod 7654 / sandbox 17654).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gateway_port: Option<u16>,
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

    pub fn effective_gateway_port(&self) -> u16 {
        self.gateway_port.unwrap_or(if is_test_sandbox() {
            DEFAULT_SANDBOX_GATEWAY_PORT
        } else {
            DEFAULT_PROD_GATEWAY_PORT
        })
    }
}

pub(super) fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Trim and expand `~` / `~/…` so Settings path fields match the on-disk repo.
pub fn expand_user_path(raw: &str) -> PathBuf {
    let s = raw.trim();
    if s == "~" {
        return home_dir();
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    PathBuf::from(s)
}

pub(super) fn default_workbench_root() -> PathBuf {
    home_dir().join("Code")
}

pub(super) fn default_knowledge_root() -> PathBuf {
    home_dir().join("Code")
}

pub fn default_cache_dir() -> PathBuf {
    home_dir().join(".cache").join("lulu-spark")
}

pub(super) fn default_github_user_url() -> String {
    DEFAULT_GITHUB_USER_URL.to_string()
}

pub(super) fn default_assistant_engine() -> String {
    "host".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            workbench_root: default_workbench_root(),
            knowledge_root: default_knowledge_root(),
            cache_dir: default_cache_dir(),
            github_user_url: default_github_user_url(),
            workbench_github_repo_url: String::new(),
            assistant_engine: default_assistant_engine(),
            llm: Vec::new(),
            http_port: None,
            mcp_port: None,
            gateway_port: None,
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

/// True when an entry selects the only supported Host platform.
pub fn is_supported_host_llm_entry(entry: &LlmSettingsEntry) -> bool {
    super::llm::normalize_engine_value(&entry.engine_type) == Some("host")
        && entry.platform.trim().eq_ignore_ascii_case(HOST_LLM_PLATFORM)
}
