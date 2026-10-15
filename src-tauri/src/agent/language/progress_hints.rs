//! Copy for the short progress hints shown while a Turn is running.

use super::catalog::{fill_text, read_text};
use super::Language;

/// The request has gone out and nothing has come back yet.
pub fn describe_requesting(language: Language) -> &'static str {
    read_text(language, "progress.requesting")
}

/// A tool call the model asked for is being run.
pub fn describe_calling_tool(language: Language, name: &str) -> String {
    fill_text(language, "progress.calling_tool", &[("name", name)])
}

/// The model is naming a tool call it is still writing.
pub fn describe_preparing_tool(language: Language, name: &str) -> String {
    fill_text(language, "progress.preparing_tool", &[("name", name)])
}

/// The model has started a tool call but not named it yet.
pub fn describe_preparing(language: Language) -> &'static str {
    read_text(language, "progress.preparing")
}

/// The reply is arriving; `preview` is its latest tail.
pub fn describe_receiving_preview(language: Language, preview: &str) -> String {
    fill_text(language, "progress.receiving_preview", &[("preview", preview)])
}

/// A response is arriving but has no content to preview yet.
pub fn describe_receiving(language: Language) -> &'static str {
    read_text(language, "progress.receiving")
}

pub fn describe_thinking(language: Language) -> &'static str {
    read_text(language, "progress.thinking")
}

/// Retry number `attempt` of at most `max`.
pub fn describe_retrying(language: Language, attempt: usize, max: usize) -> String {
    fill_text(
        language,
        "progress.retrying",
        &[("attempt", &attempt.to_string()), ("max", &max.to_string())],
    )
}
