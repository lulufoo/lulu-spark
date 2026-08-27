#[test]
fn read_later_assistant_independent_window_is_retired() {
    let src = include_str!("../../lib.rs");
    assert!(
        !src.contains("READ_LATER_ASSISTANT_LABEL"),
        "READ_LATER_ASSISTANT_LABEL must be retired with the independent window"
    );
    assert!(
        !src.contains("fn read_later_assistant_entry_path"),
        "read_later_assistant_entry_path must be retired with the independent window"
    );
    assert!(
        !src.contains("fn read_later_assistant_spec"),
        "read_later_assistant_spec must be retired with the independent window"
    );
    assert!(
        !src.contains("struct ReadLaterAssistantWindowSpec"),
        "ReadLaterAssistantWindowSpec must be retired with the independent window"
    );
    assert!(
        !src.contains("read-later-assistant.html"),
        "lib.rs must not reference read-later-assistant.html entry"
    );
    assert!(
        !src.contains("fn create_read_later_assistant_window"),
        "create_read_later_assistant_window must be retired"
    );
}
