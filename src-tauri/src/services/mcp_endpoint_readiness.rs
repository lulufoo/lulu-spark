//! Host-managed MCP endpoint discovery / readiness.
//!
//! Transport is provided to Cursor runners only after Workbench local HTTP and
//! the knowledge-MCP sidecar pass health probes. A static localhost URL alone
//! is never treated as ready. Unknown business keys reject explicitly — never
//! silently degrade to “no MCP but claimed injected”.

use std::collections::BTreeMap;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::services::mcp_server_registry::{self, HttpMcpTransport, McpServerLookupError};

/// Candidate endpoints + the transport to expose when both probes succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpEndpointCandidates {
    pub workbench_http_base: String,
    pub knowledge_mcp_base: String,
    pub transport: HttpMcpTransport,
}

/// Transports proven ready by health probes (not mere URL strings).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadyMcpTransports {
    pub transports: Vec<HttpMcpTransport>,
}

/// Explicit readiness failure — recoverable; do not start Cursor agent injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpEndpointReadinessError {
    WorkbenchHttpNotReady,
    KnowledgeMcpNotReady,
    NotWorkbenchService,
    Recoverable(String),
}

/// Errors when resolving ready transports for a business key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadyTransportError {
    Lookup(McpServerLookupError),
    Readiness(McpEndpointReadinessError),
}

/// SDK `mcpServers` HTTP entry shape (`type` / `url` / `headers`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SdkHttpMcpServer {
    #[serde(rename = "type")]
    pub transport_type: String,
    pub url: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
}

const PROBE_TIMEOUT: Duration = Duration::from_millis(500);

fn trim_trailing_slash(base: &str) -> &str {
    base.trim_end_matches('/')
}

fn get_json(url: &str) -> Result<(u16, Value), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(PROBE_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let response = client.get(url).send().map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let body = response.json::<Value>().map_err(|e| e.to_string())?;
    Ok((status, body))
}

fn probe_workbench_http(base: &str) -> Result<(), McpEndpointReadinessError> {
    let url = format!("{}/api/status", trim_trailing_slash(base));
    let (status, body) = match get_json(&url) {
        Ok(v) => v,
        Err(msg) => {
            return Err(if is_connect_failure(&msg) {
                McpEndpointReadinessError::WorkbenchHttpNotReady
            } else {
                McpEndpointReadinessError::Recoverable(msg)
            });
        }
    };
    if status != 200 {
        return Err(McpEndpointReadinessError::WorkbenchHttpNotReady);
    }
    let ok = body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if !ok || body.get("http_port").is_none() {
        return Err(McpEndpointReadinessError::NotWorkbenchService);
    }
    Ok(())
}

fn probe_knowledge_mcp(base: &str) -> Result<(), McpEndpointReadinessError> {
    let url = format!("{}/health", trim_trailing_slash(base));
    let (status, body) = match get_json(&url) {
        Ok(v) => v,
        Err(msg) => {
            return Err(if is_connect_failure(&msg) {
                McpEndpointReadinessError::KnowledgeMcpNotReady
            } else {
                McpEndpointReadinessError::Recoverable(msg)
            });
        }
    };
    if status != 200 {
        return Err(McpEndpointReadinessError::KnowledgeMcpNotReady);
    }
    let ok = body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let mcp = body.get("mcp").and_then(|v| v.as_str()).unwrap_or("");
    if !ok || mcp.trim().is_empty() {
        return Err(McpEndpointReadinessError::KnowledgeMcpNotReady);
    }
    Ok(())
}

fn is_connect_failure(msg: &str) -> bool {
    let lower = msg.to_ascii_lowercase();
    lower.contains("connection refused")
        || lower.contains("error trying to connect")
        || lower.contains("timed out")
        || lower.contains("timeout")
}

/// Health-probe candidate endpoints; return ready transports or an explicit error.
pub fn probe_mcp_endpoint_readiness(
    candidates: &McpEndpointCandidates,
) -> Result<ReadyMcpTransports, McpEndpointReadinessError> {
    probe_workbench_http(&candidates.workbench_http_base)?;
    probe_knowledge_mcp(&candidates.knowledge_mcp_base)?;
    Ok(ReadyMcpTransports {
        transports: vec![candidates.transport.clone()],
    })
}

/// Look up a business key, probe Host endpoints, and return ready transports.
/// Unknown keys reject before any “injected” claim.
pub fn ready_transports_for_business_key(
    key: &str,
    http_port: u16,
    mcp_port: u16,
) -> Result<ReadyMcpTransports, ReadyTransportError> {
    let config = mcp_server_registry::lookup(key).map_err(ReadyTransportError::Lookup)?;
    let mut transport = config.http_transport().clone();
    // Bind the transport URL to the probed MCP port (candidate → ready only after probe).
    transport.url = format!("http://127.0.0.1:{mcp_port}/mcp");
    let candidates = McpEndpointCandidates {
        workbench_http_base: format!("http://127.0.0.1:{http_port}"),
        knowledge_mcp_base: format!("http://127.0.0.1:{mcp_port}"),
        transport,
    };
    probe_mcp_endpoint_readiness(&candidates).map_err(ReadyTransportError::Readiness)
}

/// Pure mapping: `ReadyMcpTransports` → Cursor SDK `mcpServers` shape.
pub fn map_to_sdk_mcp_servers(ready: &ReadyMcpTransports) -> BTreeMap<String, SdkHttpMcpServer> {
    let mut out = BTreeMap::new();
    for t in &ready.transports {
        out.insert(
            t.name.clone(),
            SdkHttpMcpServer {
                transport_type: "http".to_string(),
                url: t.url.clone(),
                headers: t.headers.clone(),
            },
        );
    }
    out
}

#[cfg(test)]
#[path = "../unit-tests/services/mcp_endpoint_readiness_tests.rs"]
mod tests;
