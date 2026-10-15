//! Text sent to the model. One Markdown file per prompt, embedded at compile time.
//!
//! Text shown to the user lives in `agent::language`, not here.

/// Host spark chat system prompt (not toml / notes store).
///
/// SSOT for key-only Binding Set → `Binding.prompt` → turn system message.
/// Registry `capability_description` stays a short MCP identity string and is
/// not used as the LLM system prompt.
pub const SPARK_HOST_SYSTEM_PROMPT: &str = include_str!("system.md");

/// Sent as a trailing user message on the last LLM call of a Turn (`tool_choice: "none"`).
pub const MAX_STEPS_PROMPT: &str = include_str!("max_steps.md");

/// System prompt of the summary request that replaces the active turns when the context is full.
pub const SUMMARY_SYSTEM_PROMPT: &str = include_str!("compress_summary.md");

/// Result text of a tool call the model asked for on the last call; the call is not run.
pub fn read_skipped_tool_text() -> &'static str {
    include_str!("skipped_tool.md").trim_end()
}

#[cfg(test)]
#[path = "../../unit-tests/agent/prompt_tests.rs"]
mod tests;
