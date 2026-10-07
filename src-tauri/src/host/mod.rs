//! Crate-root host process helpers (ports, window, MCP runtime, live NIC).

mod port;
mod runtime;
mod window;
pub mod lan_ip;

pub use port::{decide_spawn, wait_for_port, SpawnDecision};
pub use runtime::EmbeddedMcpRuntime;

#[cfg(not(test))]
pub(crate) use window::{create_main_window, present_main_window};
