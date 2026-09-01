//! MCP adapter types and in-process tool-route tables.

use std::net::SocketAddr;
use std::thread::JoinHandle;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

/// V5/Topic2 close-gate evidence: dual listen + session initialize/tools/list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseGateReport {
    pub mcp_port: u16,
    pub sidecar_port: u16,
    pub scene_slot: String,
    pub dual_listen_observed: bool,
    pub initialize_ok: bool,
    pub tools_list_ok: bool,
    pub tool_names: Vec<String>,
}

/// Failure of the P1 close gate (blocks Node spawn hard-cut).
#[derive(Debug)]
pub enum CloseGateError {
    UnregisteredSlot(String),
    DualListenNotObservable(String),
    InitializeFailed(String),
    ToolsListFailed(String),
    Runtime(String),
}

impl std::fmt::Display for CloseGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnregisteredSlot(slot) => write!(f, "unregistered scene_slot: {slot}"),
            Self::DualListenNotObservable(msg) => {
                write!(f, "dual listen not observable: {msg}")
            }
            Self::InitializeFailed(msg) => write!(f, "initialize failed: {msg}"),
            Self::ToolsListFailed(msg) => write!(f, "tools/list failed: {msg}"),
            Self::Runtime(msg) => write!(f, "close-gate runtime: {msg}"),
        }
    }
}

impl std::error::Error for CloseGateError {}

/// Observable MCP tool success (Node: `{ content: [{ type: "text", text }] }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolResult {
    pub content_text: String,
}

/// Observable MCP tool error (Node: `{ content: [{ type: "text", text: "HTTP …" }], isError: true }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolError {
    pub content_text: String,
}

/// In-process Services call for one allowlisted MCP tool.
pub type ToolInvoke = fn(&Value) -> Value;

/// One allowlisted tool and its in-process Services invoke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRoute {
    pub name: String,
    pub description: String,
    /// True only when the tool changes no Host-managed data.
    pub read_only: bool,
    /// A destructive mutation hint for MCP clients. Meaningful only for writes.
    pub destructive: bool,
    /// JSON Schema passed to MCP `tools/list` and, later, to model tool definitions.
    pub input_schema: Value,
    pub invoke: ToolInvoke,
}

/// Authoritative slot→tool routing table for a registered `scene_slot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotToolTable {
    pub scene_slot: String,
    pub tools: Vec<ToolRoute>,
}

/// Observable tool descriptor for `tools/list` contract checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub name: String,
}

pub(crate) const REGISTERED_SCENE_SLOTS: &[&str] = &["workbench", "cursor_ide"];

pub struct McpRuntimeConfig {
    pub bind_addr: SocketAddr,
}

/// Live MCP runtime handle. Prefer [`stop_embedded_mcp_runtime`] on Host teardown;
/// dropping also cancels and joins the OS thread.
pub struct McpRuntimeHandle {
    pub(crate) local_addr: SocketAddr,
    pub(crate) cancel: CancellationToken,
    pub(crate) join: Option<JoinHandle<()>>,
}

/// Backward-compatible alias from the T1 listen scaffold.
impl McpRuntimeHandle {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

impl Drop for McpRuntimeHandle {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Failure starting the Host MCP listen scaffold.
#[derive(Debug)]
pub enum McpStartError {
    BindFailed(String),
    Runtime(String),
}

impl std::fmt::Display for McpStartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindFailed(msg) => write!(f, "MCP bind failed: {msg}"),
            Self::Runtime(msg) => write!(f, "MCP runtime error: {msg}"),
        }
    }
}

impl std::error::Error for McpStartError {}

/// Failure stopping the embedded MCP runtime.
#[derive(Debug)]
pub enum McpStopError {
    JoinFailed(String),
}

impl std::fmt::Display for McpStopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JoinFailed(msg) => write!(f, "MCP stop/join failed: {msg}"),
        }
    }
}

impl std::error::Error for McpStopError {}
