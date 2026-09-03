use std::path::PathBuf;

fn lib_rs_source() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("read src-tauri/src/lib.rs")
}

#[test]
fn generate_handler_registers_message_center_commands() {
    let src = lib_rs_source();
    assert!(
        src.contains("commands::read::get_message_channel_unread"),
        "generate_handler must register get_message_channel_unread"
    );
    assert!(
        src.contains("commands::write::mark_message_channel_read"),
        "generate_handler must register mark_message_channel_read"
    );
}

#[test]
fn setup_adapts_l4_notify_to_desktop_event() {
    let src = lib_rs_source();
    let setup = src
        .find(".setup")
        .expect("lib.rs must adapt the notify port during .setup");
    let handler = src
        .find("set_changed_handler")
        .expect("setup must attach L4 set_changed_handler");
    let event = src
        .find("message-center:changed")
        .expect("setup must emit desktop event message-center:changed");
    assert!(
        handler > setup && event > setup,
        "L4 notify adapter and message-center:changed must live in .setup"
    );
    assert!(
        src.contains("emit(") && src.contains("message-center:changed"),
        "adapter must emit message-center:changed"
    );
}

#[test]
fn does_not_register_a_message_center_invoke() {
    let src = lib_rs_source();
    for forbidden in [
        "register_message_center",
        "register_message_channel",
        "register_consumer",
        "commands::read::register",
        "commands::write::register",
    ] {
        assert!(
            !src.contains(forbidden),
            "must not add register invoke {forbidden}"
        );
    }
}
