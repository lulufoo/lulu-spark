//! Host-authoritative business Profile source.
//!
//! A Profile is assembled synchronously for one stable `business_id`. Registry
//! entries are only MCP candidates; endpoint readiness is proved before the
//! resulting `mcp_servers` are exposed.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::config::settings;
use crate::services::mcp_endpoint_readiness::{
    self, McpEndpointCandidates, McpEndpointReadinessError, SdkHttpMcpServer,
};
use crate::services::mcp_server_registry::{self, McpServerLookupError};

const BUSINESS_CWDS_DIR: &str = "agent/business-cwds";

/// Complete business-scoped configuration consumed by the Cursor create path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BusinessProfileSnapshot {
    pub business_id: String,
    pub model: String,
    pub cwd: PathBuf,
    pub mcp_servers: BTreeMap<String, SdkHttpMcpServer>,
}

/// Explicit failure from synchronous Profile assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileQueryError {
    InvalidBusinessId,
    Settings(String),
    MissingModel,
    Cwd(String),
    Registry(McpServerLookupError),
    Readiness(McpEndpointReadinessError),
    EmptyMcpServers,
}

impl fmt::Display for ProfileQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBusinessId => write!(f, "invalid business_id"),
            Self::Settings(message) => write!(f, "profile settings unavailable: {message}"),
            Self::MissingModel => write!(f, "cursor settings model is empty"),
            Self::Cwd(message) => write!(f, "business cwd unavailable: {message}"),
            Self::Registry(error) => write!(f, "business MCP registry lookup failed: {error:?}"),
            Self::Readiness(error) => write!(f, "business MCP readiness failed: {error:?}"),
            Self::EmptyMcpServers => write!(f, "business MCP servers are empty"),
        }
    }
}

impl std::error::Error for ProfileQueryError {}

fn normalize_business_id(raw: &str) -> Result<String, ProfileQueryError> {
    let business_id = raw.trim();
    if business_id.is_empty()
        || business_id.contains('/')
        || business_id.contains('\\')
        || business_id.contains("..")
    {
        return Err(ProfileQueryError::InvalidBusinessId);
    }
    Ok(business_id.to_string())
}

fn resolve_business_cwd(cache_dir: &Path, business_id: &str) -> Result<PathBuf, ProfileQueryError> {
    if cache_dir.as_os_str().is_empty() {
        return Err(ProfileQueryError::Cwd("cache_dir is empty".into()));
    }

    let cwd = cache_dir.join(BUSINESS_CWDS_DIR).join(business_id);
    fs::create_dir_all(&cwd).map_err(|error| ProfileQueryError::Cwd(error.to_string()))?;
    if !cwd.is_dir() {
        return Err(ProfileQueryError::Cwd(format!(
            "path is not a directory: {}",
            cwd.display()
        )));
    }
    Ok(cwd)
}

/// Query the complete Host business Profile for `business_id`.
///
/// The registry is consulted for an MCP candidate, then the existing Host
/// readiness probes must pass before `mcp_servers` are mapped for delivery.
/// No session or instance fields participate in this snapshot.
pub fn query_business_profile(
    business_id: &str,
) -> Result<BusinessProfileSnapshot, ProfileQueryError> {
    let business_id = normalize_business_id(business_id)?;
    let registry_config =
        mcp_server_registry::lookup(&business_id).map_err(ProfileQueryError::Registry)?;

    let app_settings =
        settings::load().map_err(|error| ProfileQueryError::Settings(error.to_string()))?;
    let model = settings::llm_entry_by_type(&app_settings.llm, "cursor")
        .map(|entry| entry.model.trim().to_string())
        .filter(|model| !model.is_empty())
        .ok_or(ProfileQueryError::MissingModel)?;
    let cwd = resolve_business_cwd(&app_settings.cache_dir, &business_id)?;

    let mut transport = registry_config.http_transport().clone();
    transport.url = format!(
        "http://127.0.0.1:{}/mcp/{business_id}",
        app_settings.effective_mcp_port()
    );
    let candidates = McpEndpointCandidates {
        workbench_http_base: format!("http://127.0.0.1:{}", app_settings.effective_http_port()),
        host_mcp_base: format!("http://127.0.0.1:{}", app_settings.effective_mcp_port()),
        transport,
    };
    let ready = mcp_endpoint_readiness::probe_mcp_endpoint_readiness(&candidates)
        .map_err(ProfileQueryError::Readiness)?;
    let mcp_servers = mcp_endpoint_readiness::map_to_sdk_mcp_servers(&ready);
    if mcp_servers.is_empty() {
        return Err(ProfileQueryError::EmptyMcpServers);
    }

    Ok(BusinessProfileSnapshot {
        business_id,
        model,
        cwd,
        mcp_servers,
    })
}
