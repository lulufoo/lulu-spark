//! Session persistence under `{cache_dir}/agent-exec/sessions/{session_id}/`.

mod binding;
mod catalog;
mod live;
mod redact;
mod schema;
mod session_db;
mod step_codec;
mod store;
mod turn_store;
mod types;

pub use binding::*;
pub use live::*;
pub use redact::*;
pub use store::*;
pub use types::*;

#[cfg(test)]
#[path = "../../unit-tests/agent/session_store_sqlite.rs"]
mod session_store_sqlite_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/step_response_tests.rs"]
mod step_response_tests;
