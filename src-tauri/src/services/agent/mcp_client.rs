//! Blocking bridge from the Host Agent Loop to an HTTP MCP server.
//!
//! The Agent Loop is synchronous at its boundary, while `rmcp` is async. Each
//! operation therefore owns a short-lived current-thread runtime and a
//! Streamable HTTP MCP session. The server's `tools/list` response remains the
//! authority for model-facing tool definitions and read/write hints.

use std::collections::{BTreeMap, BTreeSet, HashMap};
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
use serde_json::{json, Value};

use crate::services::mcp_server_registry::McpServerConfig;

const CLIENT_NAME: &str = "workbench-host-agent";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const RMCP_RESERVED_HEADERS: &[&str] = &["accept", "mcp-session-id", "last-event-id"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpClientError {
    Runtime(String),
    Header(String),
    Connect(String),
    ListTools(String),
    InvalidArguments(String),
    CallTool(String),
}

impl fmt::Display for McpClientError {
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

impl std::error::Error for McpClientError {}

/// Model-facing function definitions derived from a trusted local MCP server.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCatalog {
    pub definitions: Vec<Value>,
    names: BTreeSet<String>,
    read_only_names: BTreeSet<String>,
    input_schemas: BTreeMap<String, Value>,
}

impl ToolCatalog {
    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn is_mutating(&self, name: &str) -> bool {
        self.contains(name) && !self.read_only_names.contains(name)
    }

    /// Validate model arguments against the JSON Schema published by the MCP
    /// server before a `tools/call` request crosses the process boundary.
    pub fn validate_arguments(&self, name: &str, arguments: &Value) -> Result<(), McpClientError> {
        let schema = self
            .input_schemas
            .get(name)
            .ok_or_else(|| McpClientError::InvalidArguments(format!("tool '{name}' is not discovered")))?;
        validate_json_schema(schema, arguments, "$").map_err(McpClientError::InvalidArguments)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

fn transport_headers(config: &McpServerConfig) -> Result<HashMap<HeaderName, HeaderValue>, McpClientError> {
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
            .map_err(|error| McpClientError::Header(format!("{name}: {error}")))?;
        let value = HeaderValue::from_str(value)
            .map_err(|error| McpClientError::Header(format!("{name}: {error}")))?;
        headers.insert(name, value);
    }
    Ok(headers)
}

fn transport_config(
    config: &McpServerConfig,
) -> Result<StreamableHttpClientTransportConfig, McpClientError> {
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

fn openai_tool_definition(tool: &Tool) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description.as_deref().unwrap_or_default(),
            "parameters": Value::Object((*tool.input_schema).clone()),
        }
    })
}

fn catalog_from_tools(tools: Vec<Tool>) -> ToolCatalog {
    let mut definitions = Vec::with_capacity(tools.len());
    let mut names = BTreeSet::new();
    let mut read_only_names = BTreeSet::new();
    let mut input_schemas = BTreeMap::new();

    for tool in tools {
        let name = tool.name.to_string();
        if tool
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.read_only_hint)
            .unwrap_or(false)
        {
            read_only_names.insert(name.clone());
        }
        input_schemas.insert(name.clone(), Value::Object((*tool.input_schema).clone()));
        names.insert(name);
        definitions.push(openai_tool_definition(&tool));
    }

    ToolCatalog {
        definitions,
        names,
        read_only_names,
        input_schemas,
    }
}

fn validate_json_schema(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    if let Some(expected) = schema.get("const") {
        if value != expected {
            return Err(format!("{path} must equal {expected}"));
        }
    }

    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let object = value
                .as_object()
                .ok_or_else(|| format!("{path} must be a JSON object"))?;
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            for required in schema
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if !object.contains_key(required) {
                    return Err(format!("{path} is missing required property '{required}'"));
                }
            }
            if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                if let Some(unknown) = object.keys().find(|key| !properties.contains_key(*key)) {
                    return Err(format!("{path} contains unsupported property '{unknown}'"));
                }
            }
            for (name, property_schema) in properties {
                if let Some(property) = object.get(&name) {
                    validate_json_schema(&property_schema, property, &format!("{path}.{name}"))?;
                }
            }
        }
        Some("array") => {
            let items = value
                .as_array()
                .ok_or_else(|| format!("{path} must be an array"))?;
            if let Some(item_schema) = schema.get("items") {
                for (index, item) in items.iter().enumerate() {
                    validate_json_schema(item_schema, item, &format!("{path}[{index}]"))?;
                }
            }
        }
        Some("string") if !value.is_string() => {
            return Err(format!("{path} must be a string"));
        }
        Some("boolean") if !value.is_boolean() => {
            return Err(format!("{path} must be a boolean"));
        }
        Some("number") if !value.is_number() => {
            return Err(format!("{path} must be a number"));
        }
        Some(_) | None => {}
    }
    Ok(())
}

/// Discover the active scene's model-callable tools over MCP.
pub fn discover_tools(config: &McpServerConfig) -> Result<ToolCatalog, McpClientError> {
    let transport_config = transport_config(config)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| McpClientError::Runtime(error.to_string()))?;

    runtime.block_on(async move {
        let transport = StreamableHttpClientTransport::from_config(transport_config);
        let client = client_info()
            .serve(transport)
            .await
            .map_err(|error| McpClientError::Connect(format!("{error:#}")))?;
        let result = client
            .list_tools(Default::default())
            .await
            .map_err(|error| McpClientError::ListTools(format!("{error:#}")));
        let _ = client.cancel().await;
        result.map(|tools| catalog_from_tools(tools.tools))
    })
}

/// Execute one already-discovered tool. The model argument must be a JSON object.
pub fn call_tool(
    config: &McpServerConfig,
    name: &str,
    arguments: Value,
) -> Result<ToolResult, McpClientError> {
    let arguments = arguments
        .as_object()
        .cloned()
        .ok_or_else(|| McpClientError::InvalidArguments("arguments must be a JSON object".into()))?;
    let transport_config = transport_config(config)?;
    let name = name.to_string();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| McpClientError::Runtime(error.to_string()))?;

    runtime.block_on(async move {
        let transport = StreamableHttpClientTransport::from_config(transport_config);
        let client = client_info()
            .serve(transport)
            .await
            .map_err(|error| McpClientError::Connect(format!("{error:#}")))?;
        let mut params = CallToolRequestParams::new(name);
        params.arguments = Some(arguments);
        let result = client
            .call_tool(params)
            .await
            .map_err(|error| McpClientError::CallTool(format!("{error:#}")));
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
    catalog_from_tools(tools)
}

#[cfg(test)]
pub(crate) fn headers_from_config_for_tests(
    headers: BTreeMap<String, String>,
) -> Result<HashMap<HeaderName, HeaderValue>, McpClientError> {
    transport_headers(&McpServerConfig {
        capability_description: "test".into(),
        http_transport: crate::services::mcp_server_registry::HttpMcpTransport {
            name: "test".into(),
            url: "http://127.0.0.1/test".into(),
            headers,
        },
    })
}

#[cfg(test)]
#[path = "../../unit-tests/services/agent/mcp_client_tests.rs"]
mod tests;
