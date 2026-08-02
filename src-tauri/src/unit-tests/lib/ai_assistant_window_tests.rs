use super::*;

#[test]
fn ai_assistant_label_is_ai_assistant() {
    // window_label token remains for Present/ensure payloads; independent window retired (T4).
    assert_eq!(AI_ASSISTANT_LABEL, "ai-assistant");
}

#[test]
fn ai_assistant_independent_window_entry_is_retired() {
    // T4 / L16-T: no ai-assistant.html entry path / window spec helpers.
    let src = include_str!("../../lib.rs");
    assert!(
        !src.contains("fn ai_assistant_entry_path"),
        "ai_assistant_entry_path must be retired with the independent window"
    );
    assert!(
        !src.contains("fn ai_assistant_spec"),
        "ai_assistant_spec must be retired with the independent window"
    );
    assert!(
        !src.contains("struct AiAssistantWindowSpec"),
        "AiAssistantWindowSpec must be retired with the independent window"
    );
    assert!(
        !src.contains("ai-assistant.html"),
        "lib.rs must not reference ai-assistant.html entry"
    );
    assert!(
        !src.contains("fn create_or_focus_ai_assistant_window"),
        "create_or_focus_ai_assistant_window must be retired"
    );
}
