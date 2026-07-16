use super::*;

#[test]
fn ai_assistant_label_is_ai_assistant() {
    assert_eq!(AI_ASSISTANT_LABEL, "ai-assistant");
}

#[test]
fn ai_assistant_entry_path_is_html_file() {
    assert_eq!(ai_assistant_entry_path(), "ai-assistant.html");
}

#[test]
fn ai_assistant_spec_matches_acceptance() {
    let spec = ai_assistant_spec();
    assert_eq!(spec.label, "ai-assistant");
    assert_eq!(spec.entry, "ai-assistant.html");
    assert!(spec.always_on_top);
}
