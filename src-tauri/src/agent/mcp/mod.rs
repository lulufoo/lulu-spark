//! Blocking bridge from the Host Agent Loop to an HTTP MCP server.
//!
//! Transport only: discover (`tools/list`) and call (`tools/call`). Catalog
//! types live in `agent::tools::catalog`.

use std::collections::{BTreeMap, HashMap};
use std::fmt;

use axum::http::{HeaderName, HeaderValue};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation, Tool},
    transport::{
        StreamableHttpClientTransport,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde_json::Value;

use crate::mcp_host::registry::McpServerConfig;

use super::tools::catalog::{self, ToolCatalog, ToolResult};

const CLIENT_NAME: &str = "workbench-host-agent";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RMCP_RESERVED_HEADERS: &[&str] = &["accept", "mcp-session-id", "last-event-id"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpError {
    Runtime(String),
    Header(String),
    Connect(String),
    ListTools(String),
    InvalidArguments(String),
    CallTool(String),
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runtime(message) => write!(f, "MCP runtime: {message}"),
            Self::Header(message) => write!(f, "MCP transport header: {message}"),
            Self::Connect(message) => write!(f, "MCP initialize: {message}"),
            Self::ListTools(message) => write!(f, "MCP tools/list: {message}"),
            Self::InvalidArguments(message) => write!(f, "MCP tool arguments: {message}"),
            Self::CallTool(message) => write!(f, "MCP tools/call: {message}"),
        }
    }
}

impl std::error::Error for McpError {}

fn transport_headers(config: &McpServerConfig) -> Result<HashMap<HeaderName, HeaderValue>, McpError> {
    let mut headers = HashMap::new();
    for (name, value) in &config.http_transport().headers {
        if RMCP_RESERVED_HEADERS
            .iter()
            .any(|reserved| name.eq_ignore_ascii_case(reserved))
        {
            // rmcp supplies protocol negotiation/session headers itself. The
            // seeded registry's Accept header is therefore intentionally
            // represented by the transport default rather than copied here.
            continue;
        }
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|error| McpError::Header(format!("{name}: {error}")))?;
        let value = HeaderValue::from_str(value)
            .map_err(|error| McpError::Header(format!("{name}: {error}")))?;
        headers.insert(name, value);
    }
    Ok(headers)
}

fn transport_config(
    config: &McpServerConfig,
) -> Result<StreamableHttpClientTransportConfig, McpError> {
    Ok(StreamableHttpClientTransportConfig::with_uri(
        config.http_transport().url.clone(),
    )
    .custom_headers(transport_headers(config)?))
}

fn client_info() -> ClientInfo {
    ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new(CLIENT_NAME, CLIENT_VERSION),
    )
}

/// Discover the active scene's model-callable tools over MCP.
pub fn discover_tools(config: &McpServerConfig) -> Result<ToolCatalog, McpError> {
    let transport_config = transport_config(config)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| McpError::Runtime(error.to_string()))?;

    runtime.block_on(async move {
        let transport = StreamableHttpClientTransport::from_config(transport_config);
        let client = client_info()
            .serve(transport)
            .await
            .map_err(|error| McpError::Connect(format!("{error:#}")))?;
        let result = client
            .list_tools(Default::default())
            .await
            .map_err(|error| McpError::ListTools(format!("{error:#}")));
        let _ = client.cancel().await;
        result.map(|tools| catalog::from_rmcp_tools(tools.tools))
    })
}

/// Execute one already-discovered tool. The model argument must be a JSON object.
pub fn call_tool(
    config: &McpServerConfig,
    name: &str,
    arguments: Value,
) -> Result<ToolResult, McpError> {
    let arguments = arguments
        .as_object()
        .cloned()
        .ok_or_else(|| McpError::InvalidArguments("arguments must be a JSON object".into()))?;
    let transport_config = transport_config(config)?;
    let name = name.to_string();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| McpError::Runtime(error.to_string()))?;

    runtime.block_on(async move {
        let transport = StreamableHttpClientTransport::from_config(transport_config);
        let client = client_info()
            .serve(transport)
            .await
            .map_err(|error| McpError::Connect(format!("{error:#}")))?;
        let mut params = CallToolRequestParams::new(name);
        params.arguments = Some(arguments);
        let result = client
            .call_tool(params)
            .await
            .map_err(|error| McpError::CallTool(format!("{error:#}")));
        let _ = client.cancel().await;
        result.map(|result| {
            let content = result
                .content
                .iter()
                .filter_map(|block| block.as_text().map(|text| text.text.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            ToolResult {
                content,
                is_error: result.is_error.unwrap_or(false),
            }
        })
    })
}

#[cfg(test)]
pub(crate) fn catalog_from_mcp_tools_for_tests(tools: Vec<Tool>) -> ToolCatalog {
    catalog::from_rmcp_tools(tools)
}

#[cfg(test)]
pub(crate) fn headers_from_config_for_tests(
    headers: BTreeMap<String, String>,
) -> Result<HashMap<HeaderName, HeaderValue>, McpError> {
    transport_headers(&McpServerConfig {
        capability_description: "test".into(),
        http_transport: crate::mcp_host::registry::HttpMcpTransport {
            name: "test".into(),
            url: "http://127.0.0.1/test".into(),
            headers,
        },
    })
}

#[cfg(test)]
#[path = "../../unit-tests/agent/mcp_tests.rs"]
mod tests;
