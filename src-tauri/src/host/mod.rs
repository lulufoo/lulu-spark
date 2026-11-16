//! Crate-root host process helpers (ports, window, MCP runtime, live NIC, package snapshot).

mod package;
mod port;
mod runtime;
mod system_proxy;
mod window;
pub mod lan_ip;

pub use package::{snapshot, PackageSnapshot};
pub use port::{decide_spawn, wait_for_port, SpawnDecision};
pub use runtime::EmbeddedMcpRuntime;
pub use system_proxy::{production_source, ProxyEndpoint, SystemProxySnapshot, SystemProxySource};

#[cfg(not(test))]
pub(crate) use window::{create_main_window, present_main_window};
