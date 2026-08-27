//! Sidecar HTTP loopback proxy for allowlisted MCP tools.

use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::Value;

use super::routes::build_slot_tool_table;
use super::types::{HttpMethod, McpToolError, McpToolResult};

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

/// Convert mapped success/error into `rmcp` [`CallToolResult`].
pub fn mapped_to_call_tool_result(
    mapped: Result<McpToolResult, McpToolError>,
) -> CallToolResult {
    match mapped {
        Ok(ok) => CallToolResult::success(vec![ContentBlock::text(ok.content_text)]),
        Err(err) => CallToolResult::error(vec![ContentBlock::text(err.content_text)]),
    }
}

fn json_query_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        other => Some(other.to_string()),
    }
}

fn build_get_path(api_path: &str, args: &Value) -> String {
    let Value::Object(map) = args else {
        return api_path.to_string();
    };
    let mut pairs: Vec<(String, String)> = Vec::new();
    for (key, value) in map {
        if let Some(v) = json_query_value(value) {
            pairs.push((key.clone(), v));
        }
    }
    if pairs.is_empty() {
        return api_path.to_string();
    }
    let query = pairs
        .into_iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{api_path}?{query}")
}

/// Proxy one allowlisted tool call to Sidecar HTTP loopback and map the response.
///
/// Outer `Err` = protocol/routing failure (unknown slot/tool).
/// Inner `Result` = Node-equivalent tool success vs tool error.
pub async fn proxy_tool_call(
    sidecar_base_url: &str,
    slot: &str,
    tool: &str,
    args: Value,
) -> Result<Result<McpToolResult, McpToolError>, String> {
    let table = build_slot_tool_table(slot)
        .ok_or_else(|| format!("unregistered scene_slot: {slot}"))?;
    let route = table
        .tools
        .iter()
        .find(|r| r.name == tool)
        .ok_or_else(|| format!("tool '{tool}' is not allowlisted for slot '{slot}'"))?;

    let base = sidecar_base_url.trim_end_matches('/');
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let response = match route.method {
        HttpMethod::Get => {
            let path = build_get_path(&route.api_path, &args);
            let url = format!("{base}{path}");
            client.get(url).send().await
        }
        HttpMethod::Post => {
            let url = format!("{base}{}", route.api_path);
            client.post(url).json(&args).send().await
        }
    };

    let response = match response {
        Ok(res) => res,
        Err(err) => {
            return Ok(Err(McpToolError {
                content_text: format!("HTTP 503: Workbench HTTP unreachable: {err}"),
            }));
        }
    };

    let status = response.status().as_u16();
    let body = response
        .bytes()
        .await
        .map_err(|e| format!("read sidecar body: {e}"))?;
    Ok(map_sidecar_response_to_mcp(status, &body))
}
