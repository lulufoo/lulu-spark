//! In-process dispatch for allowlisted MCP tools.

use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::Value;

use crate::mcp_host::catalog::resolve_route;
use super::types::{McpToolError, McpToolResult};

pub fn map_sidecar_response_to_mcp(
    http_status: u16,
    body: &[u8],
) -> Result<McpToolResult, McpToolError> {
    let text = String::from_utf8_lossy(body).into_owned();
    if (200..300).contains(&http_status) {
        Ok(McpToolResult {
            content_text: text,
        })
    } else {
        Err(McpToolError {
            content_text: format!("HTTP {http_status}: {text}"),
        })
    }
}

pub fn map_service_value_to_mcp(value: Value) -> Result<McpToolResult, McpToolError> {
    let status = value
        .get("_status")
        .and_then(|v| v.as_u64())
        .unwrap_or(200) as u16;
    let mut body = value;
    if let Some(obj) = body.as_object_mut() {
        obj.remove("_status");
    }
    let json = serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string());
    map_sidecar_response_to_mcp(status, json.as_bytes())
}

/// Convert mapped success/error into `rmcp` [`CallToolResult`].
pub fn mapped_to_call_tool_result(
    mapped: Result<McpToolResult, McpToolError>,
) -> CallToolResult {
    match mapped {
        Ok(ok) => CallToolResult::success(vec![ContentBlock::text(ok.content_text)]),
        Err(err) => CallToolResult::error(vec![ContentBlock::text(err.content_text)]),
    }
}

/// Dispatch one allowlisted tool call to its Services function and map the wire.
///
/// Catalog invocations are synchronous and can use blocking integrations. Run
/// them on Tokio's blocking pool so request handling never drops a blocking
/// client from the async MCP worker.
///
/// Outer `Err` = protocol/routing failure (unknown slot/tool).
/// Inner `Result` = Node-equivalent tool success vs tool error.
pub async fn proxy_tool_call(
    slot: &str,
    channel: &str,
    tool: &str,
    args: Value,
) -> Result<Result<McpToolResult, McpToolError>, String> {
    let route = resolve_route(slot, channel, tool)
        .ok_or_else(|| format!("tool '{tool}' is not allowlisted for slot '{slot}'"))?;
    tokio::task::spawn_blocking(move || map_service_value_to_mcp((route.invoke)(&args)))
        .await
        .map_err(|error| format!("in-process MCP tool worker failed: {error}"))
}
