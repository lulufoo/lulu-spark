//! Workbench Host MCP domain: business-key registry + embedded Streamable HTTP server.
//!
//! - [`registry`]: key → MCP connection config (URL/headers); does not listen.
//! - [`catalog`]: business groups, factory, runtime assembly.
//! - [`server`]: localhost listen on `/mcp/<scene_slot>`, tool dispatch to L4 Services.

pub mod registry;
pub mod catalog;
pub mod server;

pub use registry::{
    lookup, register, seed_defaults, HttpMcpTransport, McpServerConfig, McpServerLookupError,
    DEFAULT_HTTP_MCP_SERVER_NAME, SEEDED_BUSINESS_KEY,
};
pub use catalog::{
    build, build_routes_for_channel, catalog_groups, group_for_migrated_api, resolve_route,
};
pub use server::{
    build_channel_tool_table, build_slot_tool_table, close_gate_smoke_initialize_list,
    map_service_value_to_mcp, map_sidecar_response_to_mcp, mapped_to_call_tool_result,
    observe_dual_listen, proxy_tool_call, start_embedded_mcp_runtime, start_mcp_listener,
    stop_embedded_mcp_runtime, tools_list_for_slot, CloseGateError, CloseGateReport,
    McpRuntimeConfig, McpRuntimeHandle, McpStartError, McpStopError, McpToolError, McpToolResult,
    SlotToolTable, ToolDescriptor, ToolInvoke, ToolRoute,
};
pub(crate) use server::{is_registered_scene_slot, REGISTERED_SCENE_SLOTS};

#[cfg(test)]
#[path = "../unit-tests/mcp_host.rs"]
mod tests;
