use std::collections::BTreeSet;

use super::{fill_text, load_texts, read_text};
use crate::agent::language::Language;

/// Message keys start with one of these scene prefixes.
const KEY_PREFIXES: [&str; 3] = ["llm_error.", "turn.", "progress."];

/// Every message key written as a string literal in the code that picks copy.
fn keys_used_in_code() -> BTreeSet<String> {
    let sources = [
        include_str!("../../agent/language/llm_errors.rs"),
        include_str!("../../agent/language/turn_replies.rs"),
        include_str!("../../agent/language/progress_hints.rs"),
    ];
    let mut keys = BTreeSet::new();
    for source in sources {
        for prefix in KEY_PREFIXES {
            let opener = format!("\"{prefix}");
            let mut rest = source;
            while let Some(start) = rest.find(&opener) {
                let after = &rest[start + 1..];
                let end = after.find('"').expect("a key literal is closed");
                keys.insert(after[..end].to_string());
                rest = &after[end..];
            }
        }
    }
    keys
}

#[test]
fn the_english_file_parses_and_has_no_empty_text() {
    let texts = load_texts(Language::English);
    assert!(!texts.is_empty());
    for (key, text) in texts {
        assert!(!text.trim().is_empty(), "{key} has no text");
    }
}

#[test]
fn every_key_used_in_code_is_in_the_english_file() {
    let texts = load_texts(Language::English);
    for key in keys_used_in_code() {
        assert!(texts.contains_key(&key), "{key} is used in code but missing from en.json");
    }
}

#[test]
fn every_key_in_the_english_file_is_used_in_code() {
    let used = keys_used_in_code();
    for key in load_texts(Language::English).keys() {
        assert!(used.contains(key), "{key} is in en.json but no code uses it");
    }
}

#[test]
fn a_missing_key_shows_as_the_key_not_as_an_empty_text() {
    assert_eq!(read_text(Language::English, "no.such.key"), "no.such.key");
}

#[test]
fn variables_are_replaced_and_unknown_names_are_ignored() {
    let text = fill_text(
        Language::English,
        "llm_error.unknown_finish",
        &[("reason", "odd"), ("other", "x")],
    );
    assert!(text.contains("(odd)"), "{text}");
    assert!(!text.contains('{'), "{text}");
}

#[test]
fn a_variable_without_a_value_stays_as_it_is() {
    let text = fill_text(Language::English, "turn.mcp_unavailable", &[]);
    assert!(text.contains("{error}"), "{text}");
}

#[test]
fn the_language_code_names_its_locale_file() {
    assert_eq!(Language::English.code(), "en");
}

#[test]
fn progress_hints_keep_their_variables() {
    use crate::agent::language::{describe_receiving_preview, describe_retrying};
    let retry = describe_retrying(Language::English, 1, 2);
    assert!(retry.contains("1/2"), "{retry}");
    assert!(!retry.contains('{'), "{retry}");
    assert_eq!(describe_receiving_preview(Language::English, "你好"), "Receiving… 你好");
}
