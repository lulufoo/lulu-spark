use std::fs;
use std::path::PathBuf;

use crate::commands::read::get_message_channel_unread_json;
use crate::commands::write::mark_message_channel_read_json;
use crate::services::message_center::{self, channel_unread, mark_channel_read, produce};
use crate::test_support::{read_rs_dir, TestSandbox};

fn with_message_center_cmd<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    message_center::set_changed_handler(None);
    f();
    message_center::set_changed_handler(None);
}

fn source(rel: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|err| panic!("{rel}: {err}"))
}

#[test]
fn get_message_channel_unread_forwards_l4_channel_unread() {
    with_message_center_cmd(|| {
        assert_eq!(
            get_message_channel_unread_json("notes").expect("unread"),
            false
        );
        assert_eq!(channel_unread("notes"), false);

        produce("notes").expect("produce notes");
        let unread = get_message_channel_unread_json("notes").expect("unread after produce");
        assert!(unread);
        assert_eq!(unread, channel_unread("notes"));
        assert_eq!(
            get_message_channel_unread_json("read_later").expect("read_later"),
            false
        );
        assert_eq!(
            get_message_channel_unread_json("todos").expect("todos"),
            false
        );
    });
}

#[test]
fn mark_message_channel_read_forwards_l4_mark_channel_read() {
    with_message_center_cmd(|| {
        produce("notes").expect("produce notes");
        assert!(channel_unread("notes"));
        mark_message_channel_read_json("notes").expect("mark notes read");
        assert!(!get_message_channel_unread_json("notes").expect("unread after mark"));
        assert!(!channel_unread("notes"));
    });
}

#[test]
fn command_failure_does_not_bypass_l4_authority() {
    with_message_center_cmd(|| {
        produce("notes").expect("produce notes");
        assert!(channel_unread("notes"));

        let cmd_err = mark_message_channel_read_json("knowledge").expect_err("invalid channel");
        let l4_err = mark_channel_read("knowledge").expect_err("l4 invalid channel");
        assert_eq!(cmd_err, l4_err);
        assert!(channel_unread("notes"));
        assert!(get_message_channel_unread_json("notes").expect("still unread"));
    });
}

#[test]
fn commands_only_map_to_l4_and_do_not_touch_disk() {
    let read_src = source("src/commands/read.rs");
    let write_src = source("src/commands/write.rs");
    assert!(
        read_src.contains("message_center::channel_unread"),
        "get_message_channel_unread must forward to L4 channel_unread"
    );
    assert!(
        write_src.contains("message_center::mark_channel_read"),
        "mark_message_channel_read must forward to L4 mark_channel_read"
    );
    for src in [&read_src, &write_src] {
        assert!(
            !src.contains("atomic_json"),
            "L1-cmd must not persist; L4 owns the store"
        );
        assert!(
            !src.contains("message_center_path"),
            "L1-cmd must not touch the message-center path"
        );
        assert!(
            !src.contains("register_message")
                && !src.contains("register_consumer")
                && !src.contains("register_message_center"),
            "must not add a register invoke"
        );
    }
}

#[test]
fn production_modules_declare_tests_without_test_bodies() {
    let read_src = source("src/commands/read.rs");
    let write_src = source("src/commands/write.rs");
    assert!(
        read_src.contains("#[path = \"../unit-tests/commands/message_center.rs\"]"),
        "read.rs must declare the message_center command tests"
    );
    assert!(
        write_src.contains("#[path = \"../unit-tests/commands/message_center.rs\"]"),
        "write.rs must declare the message_center command tests"
    );
    let commands_dir = read_rs_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands"));
    assert!(
        !commands_dir.contains("#[test]"),
        "test bodies must not live in production command files"
    );
}
