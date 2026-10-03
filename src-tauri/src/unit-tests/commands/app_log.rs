use std::fs;
use std::path::PathBuf;

fn source(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(rel)).expect(rel)
}

#[test]
fn command_stamps_ui_side_and_skips_agent() {
    let cmd = source("src/commands/app_log.rs");
    let lib = source("src/lib.rs");
    assert!(cmd.contains("fn log_app_event"));
    assert!(cmd.contains("Side::Ui"));
    assert!(cmd.contains("app_log::log"));
    assert!(!cmd.contains("assistant-diagnostic"));
    assert!(!cmd.contains("os-notify-trace.jsonl"));
    assert!(
        lib.contains("commands::app_log::log_app_event"),
        "generate_handler must register log_app_event"
    );
    assert!(
        lib.contains("app_log::init") && lib.contains("app_log::shutdown"),
        "process start/end must hook Host setup and Exit"
    );
}
