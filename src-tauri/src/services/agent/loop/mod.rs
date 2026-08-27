//! Agent Loop + Host open/chat-turn core (single-flight, terminals, history caps).
//! Key-only business bindings discover model tools over MCP; no process-local
//! `tools::dispatch` path is available.

pub mod types;
mod binding;
mod flights;
mod history;
mod shell;
mod turn;

pub use crate::services::agent::session::{
    validate_binding, Binding, BindingStateSummary, SetError,
};

pub use binding::*;
pub use flights::*;
pub use history::*;
pub use shell::*;
pub use turn::*;
pub use types::*;
