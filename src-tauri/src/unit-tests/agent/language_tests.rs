use super::{describe_missing_config, Language};

#[test]
fn the_current_language_is_english() {
    assert_eq!(Language::current(), Language::English);
}

#[test]
fn a_message_is_written_per_language() {
    let text = describe_missing_config(Language::current());
    assert!(text.contains("not configured"), "{text}");
    assert!(text.is_ascii(), "English copy has no other script: {text}");
}
