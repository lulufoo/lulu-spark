use std::fs;

use super::{read_board_pat_status, remove_board_pat, write_board_pat};
use crate::config::vault::{
    self, test_clear_scope, test_clear_store_fail, test_fail_store, AuthSession, AuthUser,
    BindDevice, SecretError, SlotTicket,
};

const SECRET: &str = "lin_pat_do_not_leak_9f3c";

fn sample_auth_session() -> AuthSession {
    AuthSession {
        access_token: "access-aaa".into(),
        refresh_token: "refresh-bbb".into(),
        expires_at: 1_800_000_000,
        user: AuthUser {
            id: "user-42".into(),
            email: Some("ada@example.com".into()),
            name: Some("Ada".into()),
            avatar: None,
            provider: Some("google".into()),
        },
    }
}

fn source(rel: &str) -> String {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(rel);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("{rel}: {err}"))
}

#[test]
fn status_replace_and_delete_hide_the_token() {
    test_clear_scope();
    assert_eq!(read_board_pat_status().expect("status"), false);
    write_board_pat(&format!("  {SECRET}  ")).expect("set");
    assert_eq!(read_board_pat_status().expect("configured"), true);
    let shown = serde_json::to_string(&read_board_pat_status().expect("json")).expect("encode");
    assert!(!shown.contains(SECRET), "{shown}");
    assert_eq!(vault::get_board_pat().expect("read").as_deref(), Some(SECRET));

    write_board_pat("lin_pat_replaced").expect("replace");
    assert_eq!(
        vault::get_board_pat().expect("read").as_deref(),
        Some("lin_pat_replaced")
    );
    remove_board_pat().expect("delete");
    assert_eq!(read_board_pat_status().expect("cleared"), false);
    assert!(vault::get_board_pat().expect("read").is_none());
    test_clear_scope();
}

#[test]
fn blank_token_is_rejected_without_echoing_input() {
    test_clear_scope();
    let err = write_board_pat("   ").expect_err("blank");
    assert_eq!(err, "invalid board pat");
    assert!(!err.contains(SECRET));
    assert!(vault::read_vault().expect("vault").is_empty());
    test_clear_scope();
}

#[test]
fn save_failure_keeps_the_previous_token_out_of_the_error() {
    test_clear_scope();
    write_board_pat("lin_pat_keep").expect("set");
    test_fail_store();
    let err = write_board_pat(SECRET).expect_err("fail");
    assert!(!err.contains(SECRET), "{err}");
    assert!(matches!(
        vault::board_pat_configured(),
        Err(SecretError::Store(_))
    ));
    test_clear_store_fail();
    assert_eq!(
        vault::get_board_pat().expect("kept").as_deref(),
        Some("lin_pat_keep")
    );
    remove_board_pat().expect("cleanup");
}

#[test]
fn board_pat_does_not_disturb_other_secrets() {
    test_clear_scope();
    vault::update_vault(|vault| {
        vault.set_llm_api_key("host", Some("sk-host".into()));
        vault.set_mcp_slot(
            "spark",
            Some(SlotTicket {
                handle: "ticket".into(),
                state: "live".into(),
            }),
        );
        vault.set_devices(vec![BindDevice {
            device_id: "phone-a".into(),
            device_label: Some("Pixel".into()),
            token_hash: "ee".repeat(32),
            token_hint: Some("hint".into()),
            revoked: false,
        }]);
        vault.set_auth_session(Some(sample_auth_session()));
    })
    .expect("seed");
    write_board_pat(SECRET).expect("set");
    remove_board_pat().expect("delete");
    let vault = vault::read_vault().expect("vault");
    assert!(vault.board_pat().is_none());
    assert_eq!(vault.llm_api_key("host"), Some("sk-host"));
    assert!(vault.mcp_slot("spark").is_some());
    assert_eq!(vault.devices().len(), 1);
    assert_eq!(
        vault.auth_session().map(|session| session.user.id.as_str()),
        Some("user-42")
    );
    test_clear_scope();
}

#[test]
fn commands_are_registered_and_do_not_return_the_token() {
    let lib = source("src/lib.rs");
    let commands = source("src/commands/mod.rs");
    let src = source("src/commands/board_pat.rs");
    let read_acl = source("permissions/read-api.toml");
    let write_acl = source("permissions/write-api.toml");
    assert!(commands.contains("pub mod board_pat"));
    for name in [
        "commands::board_pat::get_board_pat_status",
        "commands::board_pat::set_board_pat",
        "commands::board_pat::delete_board_pat",
    ] {
        assert!(lib.contains(name), "{name}");
    }
    assert!(!lib.contains("commands::board_pat::get_board_pat,"));
    assert!(!lib.contains("commands::board_pat::get_board_pat\n"));
    assert!(read_acl.contains("\"get_board_pat_status\""));
    assert!(write_acl.contains("\"set_board_pat\""));
    assert!(write_acl.contains("\"delete_board_pat\""));
    assert!(!read_acl.contains("\"get_board_pat\""));
    assert!(!src.contains("println"));
    assert!(!src.contains("eprintln"));
    assert!(!src.contains("tracing"));
    assert!(src.contains("Result<bool, String>"));
    assert!(!src.contains("vault::get_board_pat"));
}
