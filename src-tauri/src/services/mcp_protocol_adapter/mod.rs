//! Host MCP Streamable HTTP listen (`rmcp`).
//!
//! Binds a localhost listener and nests `StreamableHttpService` under
//! `/mcp/<scene_slot>` for registered slots only. Unknown slots are rejected
//! at the HTTP/routing layer without creating an MCP session.
//! Sidecar `tiny_http` on `:8765` remains separate.
//! Adapter proxies tools only via HTTP loopback to `127.0.0.1:8765/api/*`.
//! MCP runs on an independent OS thread with its own tokio runtime.

mod channel_routes;
mod listen;
mod proxy;
mod routes;
mod types;

pub use channel_routes::build_channel_tool_table;
pub use listen::{
    close_gate_smoke_initialize_list, observe_dual_listen, start_embedded_mcp_runtime,
    start_embedded_mcp_runtime_with_sidecar, start_mcp_listener, stop_embedded_mcp_runtime,
};
pub use proxy::{map_sidecar_response_to_mcp, mapped_to_call_tool_result, proxy_tool_call};
pub use routes::{build_slot_tool_table, catalog_groups, tools_list_for_slot};
pub(crate) use routes::scene_slot_api;
pub(crate) use types::REGISTERED_SCENE_SLOTS;
pub use types::{
    CloseGateError, CloseGateReport, HttpMethod, McpRuntimeConfig, McpRuntimeHandle, McpStartError,
    McpStopError, McpToolError, McpToolResult, SlotToolTable, ToolDescriptor, ToolRoute,
    DEFAULT_SIDECAR_BASE_URL,
};

#[cfg(test)]
#[path = "../../unit-tests/services/mcp_protocol_adapter.rs"]
mod tests;
