//! Session persistence under `{cache_dir}/agent-exec/sessions/{session_id}/`.

mod binding;
mod catalog;
mod live;
mod recent;
mod redact;
mod schema;
mod session_db;
mod step_clock;
mod step_codec;
mod store;
mod title_search;
mod turn_store;
mod types;

pub use binding::*;
pub use live::*;
pub use recent::*;
pub use redact::*;
pub use step_clock::*;
pub use store::*;
pub use title_search::*;
pub use types::*;

#[cfg(test)]
#[path = "../../unit-tests/agent/session_store_sqlite.rs"]
mod session_store_sqlite_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/step_response_tests.rs"]
mod step_response_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/step_clock_tests.rs"]
mod step_clock_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/step_extra_tests.rs"]
mod step_extra_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/session_recent_cutoff.rs"]
mod session_recent_cutoff_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/session_title_search.rs"]
mod session_title_search_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/session_touch_rule.rs"]
mod session_touch_rule_tests;
