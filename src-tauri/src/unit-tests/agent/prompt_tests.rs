use super::{
    read_skipped_tool_text, read_tool_timeout_text, write_tool_timeout_text, MAX_STEPS_PROMPT,
    SPARK_HOST_SYSTEM_PROMPT, SUMMARY_SYSTEM_PROMPT,
};

#[test]
fn the_skipped_tool_text_has_no_trailing_newline() {
    assert_eq!(
        read_skipped_tool_text(),
        "Not run: the LLM call limit for this turn was reached."
    );
}

#[test]
fn every_prompt_is_english_and_not_empty() {
    for prompt in [
        SPARK_HOST_SYSTEM_PROMPT,
        MAX_STEPS_PROMPT,
        SUMMARY_SYSTEM_PROMPT,
        read_skipped_tool_text(),
        read_tool_timeout_text(),
        write_tool_timeout_text(),
    ] {
        assert!(!prompt.trim().is_empty());
        assert!(prompt.is_ascii(), "non-ASCII text in a model prompt: {prompt}");
    }
}

#[test]
fn a_read_timeout_says_it_is_safe_to_retry() {
    assert!(read_tool_timeout_text().contains("safe to retry"));
}

#[test]
fn a_write_timeout_says_the_result_is_unknown() {
    assert!(write_tool_timeout_text().contains("result is unknown"));
    assert!(write_tool_timeout_text().contains("Check the file"));
}

#[test]
fn the_summary_prompt_follows_the_conversation_language() {
    assert!(SUMMARY_SYSTEM_PROMPT.contains("in the language the user used"));
}
