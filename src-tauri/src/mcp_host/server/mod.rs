//! Embedded Host MCP server: Streamable HTTP listen, tool proxy, slot tables.

mod caller;
mod listen;
mod oauth_grant;
mod oauth_log;
mod oauth_metadata;
mod oauth_register;
mod proxy;
mod request_meta;
mod slot;
mod types;

pub use listen::{
    close_gate_smoke_initialize_list, observe_dual_listen, start_embedded_mcp_runtime,
    start_mcp_listener, stop_embedded_mcp_runtime,
};
pub use proxy::{
    map_service_value_to_mcp, map_sidecar_response_to_mcp, mapped_to_call_tool_result,
    proxy_tool_call,
};
pub use slot::{
    build_channel_enabled_table, build_channel_tool_table, build_slot_tool_table,
    tools_list_for_slot,
};
#[cfg(test)]
pub(crate) use slot::is_registered_scene_slot;
#[cfg(test)]
pub(crate) use types::REGISTERED_SCENE_SLOTS;
pub use types::{
    invoke_eq, CloseGateError, CloseGateReport, McpRuntimeConfig, McpRuntimeHandle, McpStartError,
    McpStopError, McpToolError, McpToolResult, SlotToolTable, ToolDescriptor, ToolInvoke,
    ToolRoute,
};
