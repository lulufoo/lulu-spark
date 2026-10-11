//! Blocking bridge from the Host Agent Loop to an HTTP MCP server.
//!
//! Transport only: discover (`tools/list`) and call (`tools/call`). Catalog
//! types live in `agent::tools::catalog`.

use std::collections::HashMap;
#[cfg(test)]
use std::collections::BTreeMap;
use std::fmt;
use std::time::Instant;

use axum::http::{HeaderName, HeaderValue};
use rmcp::{
    model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation},
    transport::streamable_http_client::StreamableHttpClientTransportConfig,
};
#[cfg(test)]
use rmcp::model::Tool;
use serde_json::Value;

use crate::agent::diagnostics::{self, DiagnosticEvent, TraceId};
use crate::mcp_host::registry::McpServerConfig;

#[cfg(test)]
use super::tools::catalog;
use super::tools::catalog::{ToolCatalog, ToolResult};

mod connection;
mod connection_log;
mod trace_meta;
pub use connection_log::{McpTrace, ResetReason};
pub(crate) use trace_meta::{build_trace_meta, read_trace_id};

const CLIENT_NAME: &str = "spark-host-agent";
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

/// Discover the active scene's model-callable tools over the live MCP connection.
pub fn discover_tools(
    config: &McpServerConfig,
    trace: &McpTrace,
) -> Result<ToolCatalog, McpError> {
    connection::CONNECTION.list_tools(config, trace)
}

/// Close the live MCP connection, if any, and its server session. Called when
/// the Binding is reset and when the app exits; the next request handshakes
/// again. Blocks, so it must not run on an async executor thread.
pub fn release_connection(reason: ResetReason) {
    connection::CONNECTION.reset(&McpTrace::detached(), reason);
}

/// Execute one already-discovered tool over the live MCP connection. The model
/// argument must be a JSON object.
pub fn call_tool(
    config: &McpServerConfig,
    name: &str,
    arguments: Value,
    trace_id: &TraceId,
    session_id: &str,
) -> Result<ToolResult, McpError> {
    let arguments = arguments
        .as_object()
        .cloned()
        .ok_or_else(|| McpError::InvalidArguments("arguments must be a JSON object".into()))?;
    let mut params = CallToolRequestParams::new(name.to_string());
    params.arguments = Some(arguments);
    params.meta = Some(build_trace_meta(trace_id));
    let tool_name = params.name.to_string();

    let _ = diagnostics::log(
        DiagnosticEvent::point("assistant.mcp", "call.request.started", trace_id)
            .with_session_id(session_id)
            .with_bounded_text_field("tool_name", &tool_name),
    );
    let call_started = Instant::now();
    let trace = McpTrace::new(trace_id, session_id);
    let result = connection::CONNECTION.call_tool(config, params, &trace);
    let _ = diagnostics::log(
        DiagnosticEvent::timing(
            "assistant.mcp",
            "call.request.completed",
            trace_id,
            call_started.elapsed(),
        )
        .with_session_id(session_id)
        .with_bounded_text_field("tool_name", &tool_name)
        .with_static_field("outcome", if result.is_ok() { "ok" } else { "error" }),
    );

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
