use std::collections::BTreeSet;

use super::{fill_text, load_texts, read_text};
use crate::agent::language::Language;

const LANGUAGES: [Language; 2] = [Language::English, Language::ChineseSimplified];

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
fn every_file_parses_and_has_no_empty_text() {
    for language in LANGUAGES {
        let texts = load_texts(language);
        assert!(!texts.is_empty());
        for (key, text) in texts {
            assert!(!text.trim().is_empty(), "{} {key} has no text", language.code());
        }
    }
}

#[test]
fn every_key_used_in_code_is_in_every_file() {
    for language in LANGUAGES {
        let texts = load_texts(language);
        for key in keys_used_in_code() {
            assert!(
                texts.contains_key(&key),
                "{key} is used in code but missing from {}.json",
                language.code()
            );
        }
    }
}

#[test]
fn every_key_in_every_file_is_used_in_code() {
    let used = keys_used_in_code();
    for language in LANGUAGES {
        for key in load_texts(language).keys() {
            assert!(used.contains(key), "{key} is in {}.json but no code uses it", language.code());
        }
    }
}

#[test]
fn the_files_have_the_same_keys() {
    let english: BTreeSet<_> = load_texts(Language::English).keys().collect();
    let chinese: BTreeSet<_> = load_texts(Language::ChineseSimplified).keys().collect();
    assert_eq!(english, chinese);
}

/// The `{name}` placeholders in a text, sorted.
fn variables_in(text: &str) -> Vec<String> {
    let mut names: Vec<String> = text
        .split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_string()))
        .collect();
    names.sort();
    names
}

#[test]
fn a_text_has_the_same_variables_in_every_language() {
    let english = load_texts(Language::English);
    let chinese = load_texts(Language::ChineseSimplified);
    for (key, text) in english {
        assert_eq!(variables_in(text), variables_in(&chinese[key]), "{key}");
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
        "progress.replying_preview",
        &[("preview", "hi"), ("other", "x")],
    );
    assert_eq!(text, "Replying… hi");
}

#[test]
fn a_variable_without_a_value_stays_as_it_is() {
    let text = fill_text(Language::English, "progress.replying_preview", &[]);
    assert!(text.contains("{preview}"), "{text}");
}

#[test]
fn the_language_code_names_its_locale_file() {
    assert_eq!(Language::English.code(), "en");
    assert_eq!(Language::ChineseSimplified.code(), "zh-CN");
    let locales = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("locales");
    for language in LANGUAGES {
        let file = locales.join(format!("{}.json", language.code()));
        assert!(file.is_file(), "{}", file.display());
    }
}
