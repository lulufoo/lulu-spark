use super::*;

#[test]
fn read_later_assistant_label_is_read_later_assistant() {
    assert_eq!(READ_LATER_ASSISTANT_LABEL, "read-later-assistant");
}

#[test]
fn read_later_assistant_entry_path_is_html_file() {
    assert_eq!(read_later_assistant_entry_path(), "read-later-assistant.html");
}

#[test]
fn read_later_assistant_spec_matches_acceptance() {
    let spec = read_later_assistant_spec();
    assert_eq!(spec.label, "read-later-assistant");
    assert_eq!(spec.entry, "read-later-assistant.html");
    assert!(spec.always_on_top);
}
