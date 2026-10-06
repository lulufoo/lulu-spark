use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::services::mcp_oauth::{
    issue_for_slot, ledger_record, revoke_for_slot, rotate_for_slot, verify_for_slot, OAuthError,
    Slot, TicketHandle, TicketState,
};
use crate::test_support::TestSandbox;

fn reset_slots() {
    revoke_for_slot(Slot::CursorIde).expect("revoke cursor_ide");
    revoke_for_slot(Slot::Spark).expect("revoke spark");
}

fn with_cmd<F: FnOnce()>(test: F) {
    reset_slots();
    test();
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn source(rel: &str) -> String {
    fs::read_to_string(manifest_dir().join(rel)).unwrap_or_else(|err| panic!("{rel}: {err}"))
}

fn handle_from(value: &Value) -> String {
    value["handle"]
        .as_str()
        .unwrap_or_else(|| panic!("success body must be {{ handle }}, got {value}"))
        .to_string()
}

fn assert_handle_only(value: &Value) -> String {
    let obj = value
        .as_object()
        .unwrap_or_else(|| panic!("success body must be an object, got {value}"));
    assert_eq!(
        obj.len(),
        1,
        "Authorization Bearer uses only the handle string; no url / server block: {value}"
    );
    assert!(
        obj.contains_key("handle"),
        "success body must be {{ handle }}, got {value}"
    );
    let secret = handle_from(value);
    assert!(!secret.is_empty(), "handle must be non-empty");
    assert!(
        !secret.contains("http://") && !secret.contains("https://"),
        "handle must not be a url"
    );
    secret
}

fn assert_secret_absent(text: &str, secret: &str) {
    if secret.is_empty() {
        panic!("ticket secret must be non-empty");
    }
    if text.contains(secret) {
        panic!("command error must not contain the ticket secret");
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("authorization:")
        || (lower.contains("authorization") && lower.contains("bearer "))
    {
        panic!("command error must not contain a full Authorization header");
    }
}

fn plant_mcp_json(home: &Path) -> Vec<(PathBuf, String)> {
    let body = "{\"mcpServers\":{\"sentinel\":{\"url\":\"http://example.test\"}}}\n";
    let paths = [
        home.join(".cursor/mcp.json"),
        home.join("mcp.json"),
        home.join(".cursor/mcp/mcp.json"),
    ];
    let mut planted = Vec::new();
    for path in paths {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("mkdir mcp.json parent");
        }
        fs::write(&path, body).expect("plant mcp.json sentinel");
        planted.push((path, body.to_string()));
    }
    planted
}

fn assert_mcp_json_untouched(planted: &[(PathBuf, String)]) {
    for (path, original) in planted {
        let now = fs::read_to_string(path).unwrap_or_else(|err| {
            panic!(
                "command must not remove user mcp.json {}: {err}",
                path.display()
            )
        });
        assert_eq!(
            now, *original,
            "command must not rewrite user mcp.json {}",
            path.display()
        );
    }
}

fn assert_not_live(record: Option<crate::services::mcp_oauth::LedgerRecord>) {
    assert!(
        record
            .as_ref()
            .map(|row| row.state != TicketState::Live)
            .unwrap_or(true),
        "slot must not hold a Live ledger record"
    );
}

#[test]
fn issue_cursor_ide_ticket_opens_new_or_reuses_live_and_returns_handle_only() {
    with_cmd(|| {
        let first = super::issue_cursor_ide_ticket().expect("issue");
        let secret = assert_handle_only(&first);
        let record = ledger_record(Slot::CursorIde)
            .expect("ledger")
            .expect("record");
        assert_eq!(record.slot, Slot::CursorIde);
        assert_eq!(record.handle.as_str(), secret);
        assert_eq!(record.state, TicketState::Live);
        verify_for_slot(Slot::CursorIde, TicketHandle::from_secret(secret.clone()))
            .expect("verify issued");

        let reused = super::issue_cursor_ide_ticket().expect("reuse");
        assert_eq!(assert_handle_only(&reused), secret);
        verify_for_slot(Slot::CursorIde, TicketHandle::from_secret(secret))
            .expect("reuse is not rotate");
    });
}

#[test]
fn rotate_cursor_ide_ticket_voids_old_and_returns_new_handle_only() {
    with_cmd(|| {
        let old = super::issue_cursor_ide_ticket().expect("seed");
        let old_secret = assert_handle_only(&old);
        let rotated = super::rotate_cursor_ide_ticket().expect("rotate");
        let new_secret = assert_handle_only(&rotated);
        assert_ne!(old_secret, new_secret);
        assert_eq!(
            verify_for_slot(Slot::CursorIde, TicketHandle::from_secret(old_secret))
                .expect_err("old"),
            OAuthError::rejected
        );
        verify_for_slot(
            Slot::CursorIde,
            TicketHandle::from_secret(new_secret.clone()),
        )
        .expect("new");
        let record = ledger_record(Slot::CursorIde)
            .expect("ledger")
            .expect("record");
        assert_eq!(record.handle.as_str(), new_secret);
        assert_eq!(record.state, TicketState::Live);
    });
}

#[test]
fn revoke_mcp_slot_ticket_voids_either_live_slot() {
    with_cmd(|| {
        let cursor = issue_for_slot(Slot::CursorIde).expect("seed cursor");
        let spark = issue_for_slot(Slot::Spark).expect("seed spark");

        let revoked_cursor =
            super::revoke_mcp_slot_ticket("cursor_ide".to_string()).expect("revoke cursor");
        assert_eq!(revoked_cursor, json!({ "ok": true }));
        assert_not_live(ledger_record(Slot::CursorIde).expect("cursor ledger"));
        assert_eq!(
            verify_for_slot(Slot::CursorIde, cursor).expect_err("cursor revoked"),
            OAuthError::rejected
        );
        verify_for_slot(Slot::Spark, spark.clone()).expect("spark still live");

        let revoked_spark =
            super::revoke_mcp_slot_ticket("spark".to_string()).expect("revoke spark");
        assert_eq!(revoked_spark, json!({ "ok": true }));
        assert_not_live(ledger_record(Slot::Spark).expect("spark ledger"));
        assert_eq!(
            verify_for_slot(Slot::Spark, spark).expect_err("spark revoked"),
            OAuthError::rejected
        );
    });
}

#[test]
fn commands_share_mcp_oauth_entrypoints_not_test_or_ui_forks() {
    let _: fn(Slot) -> Result<TicketHandle, OAuthError> = issue_for_slot;
    let _: fn(Slot) -> Result<TicketHandle, OAuthError> = rotate_for_slot;
    let _: fn(Slot) -> Result<(), OAuthError> = revoke_for_slot;
    let _: fn() -> Result<Value, String> = super::issue_cursor_ide_ticket;
    let _: fn() -> Result<Value, String> = super::rotate_cursor_ide_ticket;
    let _: fn(String) -> Result<Value, String> = super::revoke_mcp_slot_ticket;

    let src = source("src/commands/mcp_oauth.rs");
    assert!(
        src.contains("issue_for_slot"),
        "issue_cursor_ide_ticket must call issue_for_slot"
    );
    assert!(
        src.contains("rotate_for_slot"),
        "rotate_cursor_ide_ticket must call rotate_for_slot"
    );
    assert!(
        src.contains("revoke_for_slot"),
        "revoke_mcp_slot_ticket must call revoke_for_slot"
    );
    assert!(
        !src.contains("issue_for_test")
            && !src.contains("issue_for_ui")
            && !src.contains("rotate_for_test")
            && !src.contains("revoke_for_test"),
        "commands must not open a test- or UI-only ticket API"
    );
}

#[test]
fn commands_are_declared_and_registered_in_generate_handler() {
    let commands_mod = source("src/commands/mod.rs");
    assert!(
        commands_mod.contains("pub mod mcp_oauth"),
        "commands/mod.rs must declare mcp_oauth"
    );

    let lib = source("src/lib.rs");
    for name in [
        "commands::mcp_oauth::issue_cursor_ide_ticket",
        "commands::mcp_oauth::rotate_cursor_ide_ticket",
        "commands::mcp_oauth::revoke_mcp_slot_ticket",
        "commands::mcp_oauth::get_mcp_ticket_view",
        "commands::mcp_oauth::revoke_mcp_device_ticket",
    ] {
        assert!(
            lib.contains(name),
            "lib.rs generate_handler must register {name}"
        );
    }
}

#[test]
fn reuse_existing_cursor_ide_live_ticket_does_not_rotate() {
    with_cmd(|| {
        let first = issue_for_slot(Slot::CursorIde).expect("seed");
        let issued = super::issue_cursor_ide_ticket().expect("reuse via command");
        assert_eq!(assert_handle_only(&issued), first.as_str());
        verify_for_slot(Slot::CursorIde, first).expect("same live ticket");
    });
}

#[test]
fn success_and_failure_never_write_user_mcp_json() {
    let sandbox = TestSandbox::new();
    reset_slots();
    let planted = plant_mcp_json(sandbox.config_dir());

    super::issue_cursor_ide_ticket().expect("issue");
    super::rotate_cursor_ide_ticket().expect("rotate");
    super::revoke_mcp_slot_ticket("cursor_ide".to_string()).expect("revoke cursor");
    super::revoke_mcp_slot_ticket("spark".to_string()).expect("revoke spark");
    super::revoke_mcp_slot_ticket("mobile".to_string()).expect_err("unknown slot");

    assert_mcp_json_untouched(&planted);

    let src = source("src/commands/mcp_oauth.rs");
    assert!(
        !src.contains("mcp.json"),
        "command module must not mention or write user mcp.json"
    );
}

#[test]
fn oauth_error_maps_to_command_failure_without_rewriting_mcp_json_or_ledger() {
    let sandbox = TestSandbox::new();
    reset_slots();
    let planted = plant_mcp_json(sandbox.config_dir());
    let existing = issue_for_slot(Slot::Spark).expect("seed spark");

    let err = super::revoke_mcp_slot_ticket("mobile".to_string()).expect_err("unknown slot");
    assert_eq!(err, OAuthError::slot_unknown.to_string());
    verify_for_slot(Slot::Spark, existing).expect("unknown slot must not write ledger");
    assert_mcp_json_untouched(&planted);
}

#[test]
fn command_error_strings_do_not_leak_ticket_secret() {
    with_cmd(|| {
        let issued = super::issue_cursor_ide_ticket().expect("issue");
        let secret = assert_handle_only(&issued);
        let err = super::revoke_mcp_slot_ticket("Spark".to_string())
            .expect_err("wrong-case slot");
        assert_eq!(err, OAuthError::slot_unknown.to_string());
        assert_secret_absent(&err, &secret);
        assert_secret_absent(&format!("{err:?}"), &secret);
    });
}

#[test]
fn get_mcp_ticket_view_and_device_revoke_use_shared_oauth() {
    let sandbox = TestSandbox::new();
    crate::config::vault::test_clear_scope();
    reset_slots();
    let spark = issue_for_slot(Slot::Spark).expect("wb");
    let view = super::get_mcp_ticket_view("spark".into()).expect("view");
    assert_eq!(view["state"], "live");
    assert!(view.get("handle").is_none());
    assert!(!view.to_string().contains(spark.as_str()));

    let phone = crate::services::mcp_oauth::issue_for_device("phone-cmd", Some("Pixel"))
        .expect("phone");
    let mobile = super::get_mcp_ticket_view("mobile".into()).expect("mobile");
    assert_eq!(mobile["devices"][0]["device_id"], "phone-cmd");
    assert!(!mobile.to_string().contains(phone.as_str()));
    let revoked = super::revoke_mcp_device_ticket("phone-cmd".into()).expect("revoke");
    assert_eq!(revoked, json!({ "ok": true }));
    let after = super::get_mcp_ticket_view("mobile".into()).expect("after");
    assert_eq!(after["devices"][0]["revoked"], true);
    let _ = sandbox;
}

#[test]
fn commands_do_not_implement_connect_notify_auth_code_discovery_or_pkce() {
    let src = source("src/commands/mcp_oauth.rs").to_ascii_lowercase();
    for needle in [
        "connect",
        "notify",
        "authorization_code",
        "discovery",
        "pkce",
        "code_challenge",
        "code_verifier",
    ] {
        assert!(
            !src.contains(needle),
            "command module must not implement {needle}"
        );
    }
}
