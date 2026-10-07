use std::fs;

use crate::services::mcp_oauth::{issue_for_slot, revoke_for_slot, Slot, TicketHandle};
use crate::test_support::TestSandbox;

fn source() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/mcp_ide_env.rs"
    ))
    .expect("mcp_ide_env.rs")
}

fn oauth_source() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/commands/mcp_oauth.rs"
    ))
    .expect("mcp_oauth.rs")
}

#[test]
fn issue_and_rotate_call_apply_before_returning_handle() {
    let src = oauth_source();
    assert!(
        src.contains("apply_ide_ticket_env"),
        "issue/rotate must apply env after persist"
    );
    assert!(
        src.contains("\"env_var\""),
        "issue/rotate must return the suffixed env var name"
    );
    let apply_at = src.find("apply_ide_ticket_env").expect("call");
    let payload_at = src.find("ticket_env_payload").expect("payload");
    assert!(
        apply_at < payload_at || src[apply_at..].contains("ticket_env_payload"),
        "apply must run before the handle JSON is returned"
    );
}

#[test]
fn apply_uses_launchctl_setenv_and_skips_in_sandbox() {
    let src = source();
    assert!(src.contains("launchctl"));
    assert!(src.contains("setenv"));
    assert!(src.contains("unsetenv"));
    assert!(src.contains("is_test_sandbox"));
    assert!(
        !src.contains("LaunchAgents") || src.contains("remove_file"),
        "must not write a LaunchAgent plist"
    );
    assert!(!src.contains("fs::write"), "must not write plist bytes");
}

#[test]
fn sandbox_apply_does_not_write_launch_agent_plist() {
    let sandbox = TestSandbox::new();
    let home = sandbox.config_dir().to_path_buf();
    let agents = home.join("Library/LaunchAgents");
    fs::create_dir_all(&agents).expect("agents dir");
    super::apply_ide_ticket_env(
        Slot::CursorIde,
        &TicketHandle::from_secret("sandbox-handle"),
        None,
    )
        .expect("apply in sandbox");
    let leftover = agents.join("com.lulu-spark.env.cursor.plist");
    assert!(
        !leftover.exists(),
        "sandbox apply must not create a LaunchAgent"
    );
    let _ = sandbox;
}

fn reset_ide_slots() {
    revoke_for_slot(Slot::CursorIde).expect("revoke cursor");
    revoke_for_slot(Slot::Codex).expect("revoke codex");
    revoke_for_slot(Slot::Claude).expect("revoke claude");
}

#[test]
fn live_ide_slots_only_include_generated_tickets() {
    reset_ide_slots();
    assert!(super::live_ide_slots().is_empty());
    issue_for_slot(Slot::CursorIde).expect("cursor");
    assert_eq!(super::live_ide_slots(), vec![Slot::CursorIde]);
    issue_for_slot(Slot::Codex).expect("codex");
    issue_for_slot(Slot::Claude).expect("claude");
    assert_eq!(
        super::live_ide_slots(),
        vec![Slot::CursorIde, Slot::Codex, Slot::Claude]
    );
    revoke_for_slot(Slot::Codex).expect("revoke codex");
    assert_eq!(
        super::live_ide_slots(),
        vec![Slot::CursorIde, Slot::Claude]
    );
}

#[test]
fn apply_live_skips_empty_slots_and_writes_no_plist() {
    let sandbox = TestSandbox::new();
    reset_ide_slots();
    super::apply_live_ide_ticket_envs();
    issue_for_slot(Slot::CursorIde).expect("cursor");
    super::apply_live_ide_ticket_envs();
    let agents = sandbox.config_dir().join("Library/LaunchAgents");
    assert!(
        !agents.join("com.lulu-spark.env.cursor.plist").exists(),
        "startup apply must not write a LaunchAgent"
    );
    let lib = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs");
    assert!(
        lib.contains("apply_live_ide_ticket_envs"),
        "setup must re-setenv live IDE tickets"
    );
    let _ = sandbox;
}
