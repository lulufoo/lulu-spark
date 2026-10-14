//! Copy for failed LLM calls.

use super::Language;

pub fn describe_missing_config(language: Language) -> &'static str {
    match language {
        Language::English => {
            "Host / GLM is not configured. Enter the API key and model in Settings."
        }
    }
}
