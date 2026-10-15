//! Copy for the short progress hints shown while a Turn is running. None of them names a
//! tool or counts retries; those details stay in the logs and call records.

use super::catalog::{fill_text, read_text};
use super::Language;

/// The request has gone out, or a response has started but shows nothing yet.
pub fn describe_working(language: Language) -> &'static str {
    read_text(language, "progress.working")
}

/// The model is writing a tool call.
pub fn describe_preparing_action(language: Language) -> &'static str {
    read_text(language, "progress.preparing_action")
}

/// A tool call the model asked for is being run.
pub fn describe_running_action(language: Language) -> &'static str {
    read_text(language, "progress.running_action")
}

/// The reply is arriving but has nothing to preview yet.
pub fn describe_replying(language: Language) -> &'static str {
    read_text(language, "progress.replying")
}

/// The reply is arriving; `preview` is its latest tail.
pub fn describe_replying_preview(language: Language, preview: &str) -> String {
    fill_text(language, "progress.replying_preview", &[("preview", preview)])
}

pub fn describe_thinking(language: Language) -> &'static str {
    read_text(language, "progress.thinking")
}

/// The call failed and is being sent again.
pub fn describe_retrying(language: Language) -> &'static str {
    read_text(language, "progress.retrying")
}
