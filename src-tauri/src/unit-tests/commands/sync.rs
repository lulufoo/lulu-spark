use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn source(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(rel)).unwrap_or_else(|err| panic!("{rel}: {err}"))
}

fn toml_allow(identifier: &str) -> BTreeSet<String> {
    let text = source("permissions/sync-api.toml");
    let block = text
        .split(&format!("identifier = \"{identifier}\""))
        .nth(1)
        .and_then(|s| s.split("commands.allow = [").nth(1))
        .and_then(|s| s.split(']').next())
        .unwrap_or_else(|| panic!("{identifier} commands.allow block"));
    block.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

fn manifest_allow(identifier: &str) -> BTreeSet<String> {
    let manifest: serde_json::Value =
        serde_json::from_str(&source("gen/schemas/acl-manifests.json")).expect("acl json");
    manifest["__app-acl__"]["permissions"][identifier]["commands"]["allow"]
        .as_array()
        .unwrap_or_else(|| panic!("{identifier} allow array"))
        .iter()
        .map(|cmd| cmd.as_str().expect("command name").to_string())
        .collect()
}

#[test]
fn the_command_is_registered_and_its_gate_is_managed_state() {
    let lib = source("src/lib.rs");

    assert!(lib.contains("commands::sync::sync_documents,"), "generate_handler must register sync_documents");
    assert!(
        lib.contains("commands::sync::cancel_sync_documents,"),
        "generate_handler must register cancel_sync_documents"
    );
    assert!(lib.contains("commands::sync::check_sync_documents,"), "generate_handler must register check_sync_documents");
    assert!(lib.contains("commands::sync::get_sync_progress,"), "generate_handler must register get_sync_progress");
    assert!(
        lib.contains("app.manage(services::doc_sync::SyncGate::new())"),
        "the command asks Tauri for the gate, which must be managed"
    );
}

#[test]
fn the_command_may_be_called_from_the_ui_and_the_manifest_is_in_step() {
    let allow = toml_allow("sync-api");

    assert!(allow.contains("sync_documents"), "sync-api.toml must allow sync_documents");
    assert!(allow.contains("cancel_sync_documents"), "sync-api.toml must allow cancel_sync_documents");
    assert!(allow.contains("check_sync_documents"), "sync-api.toml must allow check_sync_documents");
    assert!(allow.contains("get_sync_progress"), "sync-api.toml must allow get_sync_progress");
    assert_eq!(allow, manifest_allow("sync-api"), "sync-api.toml vs gen/schemas/acl-manifests.json");
    for other in ["read-api", "write-api", "search-api"] {
        let text = source(&format!("permissions/{other}.toml"));
        assert!(!text.contains("sync_documents"), "{other}.toml must not allow sync_documents");
        assert!(!text.contains("cancel_sync_documents"), "{other}.toml must not allow cancel_sync_documents");
        assert!(!text.contains("check_sync_documents"), "{other}.toml must not allow check_sync_documents");
        assert!(!text.contains("get_sync_progress"), "{other}.toml must not allow get_sync_progress");
    }
}

#[test]
fn the_command_is_a_thin_shell_over_the_service() {
    let text = source("src/commands/sync.rs");
    let command = text
        .split("pub async fn sync_documents")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("sync_documents body");

    assert!(command.starts_with("(gate: State<'_, SyncGate>)"), "no payload and no AppHandle: {command}");
    assert!(command.contains("spawn_blocking"), "the sync blocks and must leave the async runtime");
    assert!(command.contains("doc_sync::sync_now(&gate)"), "{command}");
    for forbidden in ["vault", "auth_session", "cloud_sync", "token", "spark_data"] {
        assert!(!command.contains(forbidden), "the shell must not reach for {forbidden}");
    }
}

#[test]
fn the_cancel_command_is_a_thin_shell_that_does_not_wait_for_the_sync() {
    let text = source("src/commands/sync.rs");
    let command = text
        .split("pub fn cancel_sync_documents")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("cancel_sync_documents body");

    assert!(command.starts_with("(gate: State<'_, SyncGate>)"), "{command}");
    assert!(command.contains("doc_sync::cancel_now(&gate)"), "{command}");
    assert!(!command.contains("spawn_blocking"), "a cancel must answer at once, while the sync still blocks");
    assert!(!command.contains("await"), "{command}");
}

#[test]
fn the_check_command_is_a_thin_read_only_shell_that_leaves_the_gate_alone() {
    let text = source("src/commands/sync.rs");
    let command = text
        .split("pub async fn check_sync_documents")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("check_sync_documents body");

    assert!(command.starts_with("()"), "no payload, no AppHandle and no gate: {command}");
    assert!(command.contains("spawn_blocking"), "the check lists the cloud and must leave the async runtime");
    assert!(command.contains("doc_sync::check_now"), "{command}");
    assert!(!command.contains("gate"), "a check must be answerable while a sync holds the gate: {command}");
}

#[test]
fn the_progress_command_reads_through_the_gate_and_never_waits_for_a_sync() {
    let text = source("src/commands/sync.rs");
    let command = text
        .split("pub fn get_sync_progress")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("get_sync_progress body");

    assert!(command.starts_with("(gate: State<'_, SyncGate>)"), "it asks Tauri for the managed gate: {command}");
    assert!(command.contains("doc_sync::progress_now(&gate)"), "{command}");
    assert!(!command.contains("spawn_blocking"), "it only reads memory, so it answers at once: {command}");
}

#[test]
fn the_service_files_hold_no_test_code() {
    for file in ["run", "gate", "summary", "journal", "now", "cancel", "check", "progress"] {
        let rel = format!("src/services/doc_sync/{file}.rs");
        let text = source(&rel);
        assert!(!text.contains("#[test]"), "{rel} must not hold tests");
        assert!(
            text.contains(&format!("#[path = \"../../unit-tests/services/doc_sync_{file}.rs\"]")),
            "{rel} must register its test file"
        );
    }
}
