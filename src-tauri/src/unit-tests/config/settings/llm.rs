use super::{llm_entry_by_type, upsert_llm_entry};
use crate::config::settings::LlmSettings;

#[test]
fn llm_entry_helpers_accept_known_categories_and_reject_cursor() {
    let mut entries = Vec::new();
    upsert_llm_entry(
        &mut entries,
        "host",
        &LlmSettings {
            model: "glm-4".into(),
            ..Default::default()
        },
    )
    .expect("host");
    upsert_llm_entry(
        &mut entries,
        "openai",
        &LlmSettings {
            model: "gpt-4.1".into(),
            ..Default::default()
        },
    )
    .expect("openai");
    assert_eq!(llm_entry_by_type(&entries, "host").expect("host").model, "glm-4");
    assert_eq!(
        llm_entry_by_type(&entries, "openai").expect("openai").model,
        "gpt-4.1"
    );
    assert!(upsert_llm_entry(&mut entries, "cursor", &LlmSettings::default()).is_err());
    assert!(llm_entry_by_type(&entries, "cursor").is_none());
}
