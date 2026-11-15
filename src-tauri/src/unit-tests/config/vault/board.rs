use super::*;
use crate::config::vault::{read_vault, update_vault, vault_from_json, vault_to_json, vault_to_toml, vault_from_toml};
use crate::test_support::{reset_vault, TestSandbox};

const PAT: &str = "lin_pat_board_scope_fixture";
const DIAGRAM: &str = "b_1234abcd";

fn credentials() -> BoardCredentialSnapshot {
    get_board_credentials().expect("credentials").expect("configured")
}

#[test]
fn board_scope_survives_reads_and_codec_round_trips() {
    let _sandbox = TestSandbox::new();
    reset_vault();
    set_board_pat(PAT).expect("save");
    let first = credentials();
    assert_eq!(first.cache_scope, credentials().cache_scope);
    assert_eq!(first.cache_scope.len(), 32);
    assert!(!format!("{first:?}").contains(PAT));
    let vault = read_vault().expect("vault");
    for decoded in [
        vault_from_json(&vault_to_json(&vault).expect("json")).expect("decode json"),
        vault_from_toml(&vault_to_toml(&vault).expect("toml")).expect("decode toml"),
    ] {
        assert_eq!(decoded.board.unwrap().cache_scope.as_deref(), Some(first.cache_scope.as_str()));
    }
    reset_vault();
}

#[test]
fn saving_the_same_pat_retires_scope_and_resets_denials() {
    let _sandbox = TestSandbox::new();
    reset_vault();
    set_board_pat(PAT).expect("save");
    let old = credentials();
    revoke_board_credentials(&old.cache_scope).expect("revoke");
    set_board_diagram_denied(&old.cache_scope, DIAGRAM, true).expect("deny");
    set_board_pat(PAT).expect("save again");
    let next = credentials();
    assert_ne!(old.cache_scope, next.cache_scope);
    assert!(!next.revoked);
    assert!(next.denied_diagrams.is_empty());
    assert_eq!(next.pat, PAT);
    reset_vault();
}

#[test]
fn removing_and_restoring_the_pat_never_restores_its_scope() {
    let _sandbox = TestSandbox::new();
    reset_vault();
    set_board_pat(PAT).expect("save");
    let old = credentials().cache_scope;
    delete_board_pat().expect("delete");
    assert!(get_board_credentials().expect("read").is_none());
    assert!(read_vault().expect("vault").board.is_none());
    set_board_pat(PAT).expect("restore");
    assert_ne!(old, credentials().cache_scope);
    reset_vault();
}

#[test]
fn legacy_credentials_are_migrated_before_the_scope_is_returned() {
    let _sandbox = TestSandbox::new();
    reset_vault();
    update_vault(|vault| vault.set_board_pat(Some(PAT.into()))).expect("legacy");
    assert!(read_vault().unwrap().board.unwrap().cache_scope.is_none());
    let scope = credentials().cache_scope;
    assert_eq!(read_vault().unwrap().board.unwrap().cache_scope.as_deref(), Some(scope.as_str()));
    assert_eq!(scope, credentials().cache_scope);
    reset_vault();
}

#[test]
fn migration_storage_failure_does_not_return_an_ephemeral_scope() {
    let mut vault = vault_from_json(r#"{"board":{"pat":"fixture"}}"#).unwrap();
    let result = ensure_board_credentials(&mut vault, |_| Err(SecretError::Store("blocked".into())));
    assert!(matches!(result, Err(SecretError::Store(_))));
}

#[test]
fn old_scope_cannot_revoke_or_deny_the_current_credentials() {
    let _sandbox = TestSandbox::new();
    reset_vault();
    set_board_pat(PAT).expect("save");
    let old = credentials().cache_scope;
    set_board_pat("lin_pat_replacement").expect("replace");
    let before = read_vault().unwrap();
    assert!(!revoke_board_credentials(&old).unwrap());
    assert!(!set_board_diagram_denied(&old, DIAGRAM, true).unwrap());
    assert_eq!(before, read_vault().unwrap());
    reset_vault();
}

#[test]
fn revocation_and_per_diagram_denials_are_persistent_and_independent() {
    let _sandbox = TestSandbox::new();
    reset_vault();
    set_board_pat(PAT).expect("save");
    let scope = credentials().cache_scope;
    assert!(set_board_diagram_denied(&scope, DIAGRAM, true).unwrap());
    assert!(set_board_diagram_denied(&scope, "b_aaaabbbb", true).unwrap());
    assert!(revoke_board_credentials(&scope).unwrap());
    let vault = read_vault().unwrap();
    for decoded in [
        vault_from_json(&vault_to_json(&vault).unwrap()).unwrap(),
        vault_from_toml(&vault_to_toml(&vault).unwrap()).unwrap(),
    ] {
        let board = decoded.board.unwrap();
        assert!(board.revoked);
        assert!(board.denied_diagrams.contains(DIAGRAM));
    }
    assert!(set_board_diagram_denied(&scope, DIAGRAM, false).unwrap());
    let current = credentials();
    assert!(current.revoked);
    assert_eq!(current.pat, PAT);
    assert!(!current.denied_diagrams.contains(DIAGRAM));
    assert!(current.denied_diagrams.contains("b_aaaabbbb"));
    reset_vault();
}

#[test]
fn failed_pat_save_and_metadata_writes_leave_the_previous_scope_active() {
    let _sandbox = TestSandbox::new();
    reset_vault();
    set_board_pat(PAT).expect("save");
    let before = credentials();
    crate::config::vault::test_fail_store();
    assert!(set_board_pat("lin_pat_new").is_err());
    assert!(delete_board_pat().is_err());
    assert!(revoke_board_credentials(&before.cache_scope).is_err());
    assert!(set_board_diagram_denied(&before.cache_scope, DIAGRAM, true).is_err());
    crate::config::vault::test_clear_store_fail();
    let after = credentials();
    assert_eq!(before, after);
    reset_vault();
}
