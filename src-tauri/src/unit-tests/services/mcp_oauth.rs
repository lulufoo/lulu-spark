use super::*;

fn reset_slots() {
    test_force_keychain_unavailable(false);
    revoke_for_slot(Slot::CursorIde).expect("revoke cursor_ide");
    revoke_for_slot(Slot::Spark).expect("revoke workbench");
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
    let workbench = issue_for_slot(Slot::Spark).expect("issue workbench");
    let cursor_ide = issue_for_slot(Slot::CursorIde).expect("issue cursor_ide");
    assert_ne!(workbench, cursor_ide);
    verify_for_slot(Slot::Spark, workbench.clone()).expect("verify workbench");
    verify_for_slot(Slot::CursorIde, cursor_ide.clone()).expect("verify cursor_ide");
    assert_eq!(
        verify_for_slot(Slot::Spark, cursor_ide).expect_err("cross-slot"),
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
    let existing = issue_for_slot(Slot::Spark).expect("issue workbench");
    assert_eq!(
        rotate_for_slot(Slot::Spark).expect_err("workbench rotate"),
        OAuthError::rejected
    );
    verify_for_slot(Slot::Spark, existing.clone()).expect("unchanged");
    let record = ledger_record(Slot::Spark)
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
    if secret.contains("spark") || secret.contains("cursor_ide") {
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
    assert_eq!(Slot::parse("spark"), Ok(Slot::Spark));
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

    let workbench = issue_for_slot(Slot::Spark).expect("issue workbench");
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

fn with_device_sandbox<F: FnOnce()>(test: F) {
    let _sandbox = crate::test_support::TestSandbox::new();
    test();
}

fn assert_hex32_token(token: &str) {
    assert_eq!(token.len(), 64, "device token must be 32 bytes as hex");
    assert!(
        token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "device token must be lowercase hex, got {token:?}"
    );
}

fn device_ledger_text() -> String {
    let path = crate::config::settings::settings_config_dir()
        .expect("config dir")
        .join("device-tickets.json");
    std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("device ledger {} must exist: {err}", path.display())
    })
}

#[test]
fn issue_for_device_returns_token_verify_round_trips_device_id() {
    with_device_sandbox(|| {
        let token = issue_for_device("phone-a", Some("Kitchen iPad")).expect("issue");
        assert_hex32_token(token.as_str());
        assert_eq!(
            verify_device_token(token.as_str()).expect("verify"),
            "phone-a"
        );
    });
}

#[test]
fn issue_for_device_again_rotates_token_and_voids_old() {
    with_device_sandbox(|| {
        let old = issue_for_device("phone-a", None).expect("first");
        let new_token = issue_for_device("phone-a", Some("relabel")).expect("rotate");
        assert_ne!(old.as_str(), new_token.as_str());
        assert_eq!(
            verify_device_token(old.as_str()).expect_err("old"),
            OAuthError::rejected
        );
        assert_eq!(
            verify_device_token(new_token.as_str()).expect("new"),
            "phone-a"
        );
    });
}

#[test]
fn revoke_for_device_rejects_that_ticket_and_leaves_others() {
    with_device_sandbox(|| {
        let keep = issue_for_device("phone-keep", Some("keep")).expect("keep");
        let drop = issue_for_device("phone-drop", Some("drop")).expect("drop");
        revoke_for_device("phone-drop").expect("revoke");
        assert_eq!(
            verify_device_token(drop.as_str()).expect_err("revoked"),
            OAuthError::rejected
        );
        assert_eq!(
            verify_device_token(keep.as_str()).expect("other live"),
            "phone-keep"
        );
    });
}

#[test]
fn list_devices_returns_id_label_revoked_without_token_or_hash() {
    with_device_sandbox(|| {
        let token = issue_for_device("phone-list", Some("Desk")).expect("issue");
        revoke_for_device("phone-list").expect("revoke");
        let listed = list_devices().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].device_id, "phone-list");
        assert_eq!(listed[0].device_label.as_deref(), Some("Desk"));
        assert!(listed[0].revoked);
        let debug = format!("{listed:?}");
        assert!(
            !debug.contains(token.as_str()),
            "list_devices must not return the token"
        );
        let ledger = device_ledger_text();
        assert!(
            !ledger.contains(token.as_str()),
            "ledger must store SHA-256 only, not the raw token"
        );
        assert!(
            ledger.contains("token_hash"),
            "ledger row must persist a token hash field"
        );
        let hint = token_hint(token.as_str());
        assert_eq!(listed[0].token_hint.as_deref(), Some(hint.as_str()));
    });
}

#[test]
fn ticket_view_masks_workbench_and_lists_mobile_without_secrets() {
    with_device_sandbox(|| {
        let workbench = issue_for_slot(Slot::Spark).expect("wb");
        let cursor = issue_for_slot(Slot::CursorIde).expect("ide");
        let phone = issue_for_device("phone-view", Some("Pixel")).expect("phone");

        let wb = ticket_view("spark").expect("wb view");
        assert_eq!(wb["channel"], "spark");
        assert_eq!(wb["state"], "live");
        assert_eq!(wb["hint"], token_hint(workbench.as_str()));
        assert!(wb.get("handle").is_none(), "workbench view must omit handle");
        let wb_text = wb.to_string();
        assert!(!wb_text.contains(workbench.as_str()));

        let ide = ticket_view("cursor_ide").expect("ide view");
        assert_eq!(ide["handle"], cursor.as_str());

        let mobile = ticket_view("mobile").expect("mobile view");
        assert_eq!(mobile["devices"][0]["device_id"], "phone-view");
        assert_eq!(mobile["devices"][0]["hint"], token_hint(phone.as_str()));
        assert!(!mobile.to_string().contains(phone.as_str()));
        assert_eq!(ticket_view("nope").expect_err("unknown"), OAuthError::slot_unknown);
    });
}

#[test]
fn slot_enum_stays_workbench_and_cursor_ide_only() {
    assert_eq!(Slot::parse("mobile"), Err(OAuthError::slot_unknown));
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mcp_oauth.rs"
    ));
    assert!(
        !src.contains("Slot::Mobile"),
        "must not add Slot::Mobile"
    );
    assert!(src.contains("issue_for_slot"));
    assert!(src.contains("verify_for_slot"));
    assert!(src.contains("revoke_for_slot"));
}

#[test]
fn device_ticket_does_not_verify_via_slot_and_skips_oauth_keychain() {
    with_device_sandbox(|| {
        reset_slots();
        let token = issue_for_device("phone-slot", None).expect("issue");
        assert_eq!(
            verify_for_slot(Slot::Spark, token.clone()).expect_err("not a slot ticket"),
            OAuthError::rejected
        );
        assert_eq!(
            verify_for_slot(Slot::CursorIde, token.clone()).expect_err("not a slot ticket"),
            OAuthError::rejected
        );
        assert_eq!(
            verify_device_token(token.as_str()).expect("device path"),
            "phone-slot"
        );
        assert_not_live(ledger_record(Slot::Spark).expect("spark"));
        assert_not_live(ledger_record(Slot::CursorIde).expect("cursor"));
    });
}

#[test]
fn empty_device_id_is_rejected() {
    with_device_sandbox(|| {
        assert_eq!(
            issue_for_device("", None).expect_err("empty"),
            OAuthError::rejected
        );
        assert_eq!(
            issue_for_device("", Some("label")).expect_err("empty with label"),
            OAuthError::rejected
        );
    });
}

#[test]
fn verify_device_token_rejects_unknown_and_revoked() {
    with_device_sandbox(|| {
        assert_eq!(
            verify_device_token("00".repeat(32).as_str()).expect_err("unknown"),
            OAuthError::rejected
        );
        let token = issue_for_device("phone-rev", None).expect("issue");
        revoke_for_device("phone-rev").expect("revoke");
        assert_eq!(
            verify_device_token(token.as_str()).expect_err("revoked"),
            OAuthError::rejected
        );
    });
}

#[test]
fn crate_exports_device_ticket_api() {
    let _: fn(&str, Option<&str>) -> Result<TicketHandle, OAuthError> = issue_for_device;
    let _: fn(&str) -> Result<String, OAuthError> = verify_device_token;
    let _: fn(&str) -> Result<(), OAuthError> = revoke_for_device;
    let _: fn() -> Result<Vec<DeviceRecord>, OAuthError> = list_devices;
}

#[test]
fn device_ticket_sources_do_not_mount_mcp_mobile_or_host_bind_tests() {
    let oauth = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mcp_oauth.rs"
    ));
    assert!(
        !oauth.contains("/mcp/mobile"),
        "T1 must not mount /mcp/mobile"
    );
    assert!(
        !oauth.contains("create_bind_payload") && !oauth.contains("current_lan_ipv4"),
        "bind / lan_ip behavior must not live in mcp_oauth.rs"
    );
}
