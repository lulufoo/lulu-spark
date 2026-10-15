//! The fixed copy itself: one JSON file per language under `src-tauri/locales/`,
//! embedded at compile time. A message is a key plus optional `{name}` variables.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::Language;

const EN_JSON: &str = include_str!("../../../locales/en.json");
const ZH_CN_JSON: &str = include_str!("../../../locales/zh-CN.json");

type Texts = HashMap<String, String>;

fn load_texts(language: Language) -> &'static Texts {
    match language {
        Language::English => {
            static TEXTS: OnceLock<Texts> = OnceLock::new();
            TEXTS.get_or_init(|| parse_texts(EN_JSON))
        }
        Language::ChineseSimplified => {
            static TEXTS: OnceLock<Texts> = OnceLock::new();
            TEXTS.get_or_init(|| parse_texts(ZH_CN_JSON))
        }
    }
}

fn parse_texts(json: &str) -> Texts {
    serde_json::from_str(json).expect("a locale file is a flat JSON object of strings")
}

/// The text for `key`. A key that is missing from the file yields the key itself, so the
/// gap shows in the message instead of leaving it empty; the tests keep the files and the
/// code in step, so this does not happen in a build that passes them.
pub(super) fn read_text(language: Language, key: &'static str) -> &'static str {
    load_texts(language).get(key).map_or(key, String::as_str)
}

/// The text for `key` with each `{name}` replaced by its value. A name the text does not
/// contain is ignored; a `{name}` with no value stays as it is.
pub(super) fn fill_text(language: Language, key: &'static str, vars: &[(&str, &str)]) -> String {
    let mut text = read_text(language, key).to_string();
    for (name, value) in vars {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
#[path = "../../unit-tests/agent/language_catalog_tests.rs"]
mod tests;
