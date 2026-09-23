//! Context percentage of the last model prompt sent in the current session.

mod argument_order;
mod compress;
mod count;
mod render;
mod usage;
mod window;

pub use compress::maybe_compress;
pub use usage::{current_context_percent, current_context_usage, record_sent_prompt};

#[cfg(test)]
#[path = "../../unit-tests/agent/context_percent.rs"]
mod context_percent_tests;

#[cfg(test)]
#[path = "../../unit-tests/agent/context_compress.rs"]
mod context_compress_tests;
