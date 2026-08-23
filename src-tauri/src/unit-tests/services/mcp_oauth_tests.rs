use super::*;

fn reset_slots() {
    test_force_keychain_unavailable(false);
    revoke_for_slot(Slot::CursorIde).expect("revoke cursor_ide");
    revoke_for_slot(Slot::Workbench).expect("revoke workbench");
}

fn assert_not_live(record: Option<LedgerRecord>) {
    assert!(
        record
            .as_ref()
            .map(|row| row.state != TicketState::Live)
            .unwrap_or(true),
        "slot must not hold a Live ledger record"
    );
}

fn assert_secret_absent_from(text: &str, secret: &str) {
    if secret.is_empty() {
        panic!("ticket secret must be non-empty");
    }
    if text.contains(secret) {
        panic!("secret must not appear in logs, errors, or formatted output");
    }
    if text.to_ascii_lowercase().contains("authorization:")
        && !text.contains("[REDACTED]")
        && text.to_ascii_lowercase().contains("bearer ")
    {
        panic!("full Authorization must not appear in formatted output");
    }
}

#[test]
fn issue_for_empty_cursor_ide_slot_creates_live_ticket() {
    reset_slots();
    let handle = issue_for_slot(Slot::CursorIde).expect("issue");
    let record = ledger_record(Slot::CursorIde)
        .expect("ledger")
        .expect("record");
    assert_eq!(record.slot, Slot::CursorIde);
    assert_eq!(record.handle, handle);
    assert_eq!(record.state, TicketState::Live);
}

#[test]
fn issue_for_slot_reuses_existing_live_handle() {
    reset_slots();
    let first = issue_for_slot(Slot::CursorIde).expect("issue");
    let second = issue_for_slot(Slot::CursorIde).expect("reuse");
    assert_eq!(first, second);
    let record = ledger_record(Slot::CursorIde)
        .expect("ledger")
        .expect("record");
    assert_eq!(record.handle, first);
    assert_eq!(record.state, TicketState::Live);
}

#[test]
fn verify_for_slot_accepts_live_handle() {
    reset_slots();
    let handle = issue_for_slot(Slot::CursorIde).expect("issue");
    verify_for_slot(Slot::CursorIde, handle).expect("verify live");
}

#[test]
fn workbench_and_cursor_ide_hold_separate_tickets() {
    reset_slots();
    let workbench = issue_for_slot(Slot::Workbench).expect("issue workbench");
    let cursor_ide = issue_for_slot(Slot::CursorIde).expect("issue cursor_ide");
    assert_ne!(workbench, cursor_ide);
    verify_for_slot(Slot::Workbench, workbench.clone()).expect("verify workbench");
    verify_for_slot(Slot::CursorIde, cursor_ide.clone()).expect("verify cursor_ide");
    assert_eq!(
        verify_for_slot(Slot::Workbench, cursor_ide).expect_err("cross-slot"),
        OAuthError::rejected
    );
    assert_eq!(
        verify_for_slot(Slot::CursorIde, workbench).expect_err("cross-slot"),
        OAuthError::rejected
    );
}

#[test]
fn revoke_for_slot_voids_live_ticket_and_verify_rejects() {
    reset_slots();
    let handle = issue_for_slot(Slot::CursorIde).expect("issue");
    revoke_for_slot(Slot::CursorIde).expect("revoke");
    assert_not_live(ledger_record(Slot::CursorIde).expect("ledger"));
    assert_eq!(
        verify_for_slot(Slot::CursorIde, handle).expect_err("revoked"),
        OAuthError::rejected
    );
}

#[test]
fn rotate_for_slot_replaces_cursor_ide_live_ticket() {
    reset_slots();
    let old = issue_for_slot(Slot::CursorIde).expect("issue");
    let new_handle = rotate_for_slot(Slot::CursorIde).expect("rotate");
    assert_ne!(old, new_handle);
    assert_eq!(
        verify_for_slot(Slot::CursorIde, old).expect_err("old"),
        OAuthError::rejected
    );
    verify_for_slot(Slot::CursorIde, new_handle.clone()).expect("new");
    let record = ledger_record(Slot::CursorIde)
        .expect("ledger")
        .expect("record");
    assert_eq!(record.handle, new_handle);
    assert_eq!(record.state, TicketState::Live);
}

#[test]
fn settings_and_tests_share_revoke_and_rotate_entrypoints() {
    let _: fn(Slot) -> Result<(), OAuthError> = revoke_for_slot;
    let _: fn(Slot) -> Result<TicketHandle, OAuthError> = rotate_for_slot;
}

#[test]
fn crate_exports_mcp_oauth_module() {
    let _: fn(Slot) -> Result<TicketHandle, OAuthError> = crate::services::mcp_oauth::issue_for_slot;
    let _: fn(Slot, TicketHandle) -> Result<(), OAuthError> =
        crate::services::mcp_oauth::verify_for_slot;
    let _: fn(Slot) -> Result<(), OAuthError> = crate::services::mcp_oauth::revoke_for_slot;
    let _: fn(Slot) -> Result<TicketHandle, OAuthError> =
        crate::services::mcp_oauth::rotate_for_slot;
    let _: fn(Slot) -> Result<Option<LedgerRecord>, OAuthError> =
        crate::services::mcp_oauth::ledger_record;
}

#[test]
fn revoke_for_slot_without_live_ticket_succeeds() {
    reset_slots();
    revoke_for_slot(Slot::CursorIde).expect("first empty revoke");
    revoke_for_slot(Slot::CursorIde).expect("second empty revoke");
}

#[test]
fn rotate_for_slot_rejects_workbench_without_new_error_or_slot_unknown() {
    reset_slots();
    let existing = issue_for_slot(Slot::Workbench).expect("issue workbench");
    assert_eq!(
        rotate_for_slot(Slot::Workbench).expect_err("workbench rotate"),
        OAuthError::rejected
    );
    verify_for_slot(Slot::Workbench, existing.clone()).expect("unchanged");
    let record = ledger_record(Slot::Workbench)
        .expect("ledger")
        .expect("record");
    assert_eq!(record.handle, existing);
    assert_eq!(record.state, TicketState::Live);
}

#[test]
fn ledger_record_is_read_only() {
    reset_slots();
    let handle = issue_for_slot(Slot::CursorIde).expect("issue");
    let first = ledger_record(Slot::CursorIde).expect("first read");
    let second = ledger_record(Slot::CursorIde).expect("second read");
    assert_eq!(first, second);
    let record = first.expect("record");
    assert_eq!(record.handle, handle);
    assert_eq!(record.state, TicketState::Live);
    verify_for_slot(Slot::CursorIde, handle).expect("still live");
}

#[test]
fn ticket_handle_debug_and_display_are_redacted() {
    reset_slots();
    let handle = issue_for_slot(Slot::CursorIde).expect("issue");
    let secret = handle.as_str().to_string();
    assert_secret_absent_from(&format!("{handle:?}"), &secret);
    assert_secret_absent_from(&format!("{handle}"), &secret);
    assert_secret_absent_from(&format!("{:?}", OAuthError::rejected), &secret);
    assert_secret_absent_from(&OAuthError::rejected.to_string(), &secret);
}

#[test]
fn ticket_face_is_bearer_secret_only_not_slot_or_jwt() {
    reset_slots();
    let handle = issue_for_slot(Slot::CursorIde).expect("issue");
    let secret = handle.as_str().to_string();
    if secret.is_empty() {
        panic!("ticket secret must be non-empty");
    }
    if secret.contains("workbench") || secret.contains("cursor_ide") {
        panic!("ticket face must not contain a slot name");
    }
    if secret.starts_with("eyJ") || secret.matches('.').count() == 2 {
        panic!("TicketHandle must not be encoded as JWT");
    }
    if secret.contains(' ') || secret.contains(':') {
        panic!("Authorization Bearer must carry only the random string");
    }
}

#[test]
fn unknown_slot_name_is_slot_unknown() {
    assert_eq!(Slot::parse("mobile"), Err(OAuthError::slot_unknown));
    assert_eq!(Slot::parse(""), Err(OAuthError::slot_unknown));
    assert_eq!(Slot::parse("Workbench"), Err(OAuthError::slot_unknown));
    assert_eq!(Slot::parse("workbench"), Ok(Slot::Workbench));
    assert_eq!(Slot::parse("cursor_ide"), Ok(Slot::CursorIde));
}

#[test]
fn verify_for_slot_rejects_missing_mismatched_and_revoked_the_same_way() {
    reset_slots();
    let missing = TicketHandle::from_secret("no-such-ticket");
    assert_eq!(
        verify_for_slot(Slot::CursorIde, missing).expect_err("missing"),
        OAuthError::rejected
    );

    let workbench = issue_for_slot(Slot::Workbench).expect("issue workbench");
    assert_eq!(
        verify_for_slot(Slot::CursorIde, workbench).expect_err("mismatch"),
        OAuthError::rejected
    );

    let live = issue_for_slot(Slot::CursorIde).expect("issue cursor_ide");
    revoke_for_slot(Slot::CursorIde).expect("revoke");
    assert_eq!(
        verify_for_slot(Slot::CursorIde, live).expect_err("revoked"),
        OAuthError::rejected
    );
}

#[test]
fn keychain_write_failure_issues_no_ticket_and_does_not_fall_back() {
    reset_slots();
    test_force_keychain_unavailable(true);
    assert_eq!(
        issue_for_slot(Slot::CursorIde).expect_err("keychain down"),
        OAuthError::keychain_unavailable
    );
    test_force_keychain_unavailable(false);
    assert_not_live(ledger_record(Slot::CursorIde).expect("ledger after failed issue"));
}

#[test]
fn keychain_read_failure_is_keychain_unavailable() {
    reset_slots();
    let _ = issue_for_slot(Slot::CursorIde).expect("issue");
    test_force_keychain_unavailable(true);
    assert_eq!(
        ledger_record(Slot::CursorIde).expect_err("read fail"),
        OAuthError::keychain_unavailable
    );
    assert_eq!(
        verify_for_slot(Slot::CursorIde, TicketHandle::from_secret("opaque"))
            .expect_err("verify read fail"),
        OAuthError::keychain_unavailable
    );
    test_force_keychain_unavailable(false);
}
