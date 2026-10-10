//! Staged files: fence grants per session, unstage revokes, Stage is LLM-only.

use std::fs;
use std::path::Path;

use crate::agent::binding::loaded_path_fence;
use crate::agent::r#loop;
use crate::agent::session;
use crate::agent::tools::{host, stage};
use crate::commands::ai_assistant::{
    create_chat_session_json, delete_chat_session_json, unstage_chat_staged_json,
};
use crate::mcp_host::registry::SEEDED_BUSINESS_KEY;
use crate::services::path_fence::validate_stage_file;
use crate::test_support::TestSandbox;
use serde_json::json;

fn with_bound_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    // Earlier tests may clear the Host registry. Re-seed before Set so this
    // fixture does not depend on leftover process state.
    crate::mcp_host::registry::seed_defaults();
    r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("bind");
    f(&sandbox);
}

/// What the `stage` tool does for an external file: validate, then register without a source.
fn stage_external(session_id: &str, file: &Path) {
    let base = loaded_path_fence().expect("fence");
    let fence = base.with_session_scratch(session_id).unwrap_or(base);
    let canon = validate_stage_file(&file.to_string_lossy(), &fence).expect("validate");
    let mut live = session::load_session(session_id).expect("load");
    live.register_staged(&canon.to_string_lossy(), None, None)
        .expect("register");
    session::save_session(&live).expect("save");
}

fn new_session() -> String {
    create_chat_session_json().expect("create")["session_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn staged_external_file_grants_exact_file_write_per_session() {
    with_bound_sandbox(|sandbox| {
        let sid_a = new_session();
        let sid_b = new_session();
        let external = sandbox.config_dir().join("external");
        fs::create_dir_all(&external).expect("external");
        let file_a = external.join("a.md");
        let file_b = external.join("b.md");
        fs::write(&file_a, "old-a").expect("a");
        fs::write(&file_b, "old-b").expect("b");
        stage_external(&sid_a, &file_a);
        stage_external(&sid_b, &file_b);
        let base = loaded_path_fence().expect("fence");
        let mut fence_a = base.with_session_scratch(&sid_a).expect("scratch a");
        let mut fence_b = base.with_session_scratch(&sid_b).expect("scratch b");
        for entry in session::load_session(&sid_a).expect("load a").list_staged() {
            fence_a.grant_staged_file(std::path::PathBuf::from(&entry.path));
        }
        for entry in session::load_session(&sid_b).expect("load b").list_staged() {
            fence_b.grant_staged_file(std::path::PathBuf::from(&entry.path));
        }
        assert!(fence_a.allows_write(&file_a));
        assert!(!fence_a.allows_write(&file_b));
        assert!(fence_b.allows_write(&file_b));
        assert!(!fence_b.allows_write(&file_a));
        let wrote = host::call(
            "write",
            &json!({
                "path": file_a.to_string_lossy(),
                "content": "new-a"
            }),
            &fence_a,
        );
        assert!(!wrote.is_error, "{}", wrote.content);
        assert_eq!(fs::read_to_string(&file_a).expect("read"), "new-a");
        let scratch_a = fence_a.session_scratch_root(&sid_a).expect("root").expect("some");
        fs::create_dir_all(&scratch_a).expect("scratch dir");
        let pad = scratch_a.join("pad.md");
        assert!(fence_a.allows_write(&pad));
        let padded = host::call(
            "write",
            &json!({ "path": pad.to_string_lossy(), "content": "from-pad" }),
            &fence_a,
        );
        assert!(!padded.is_error, "{}", padded.content);
        let copied = host::call(
            "copy",
            &json!({
                "source_path": pad.to_string_lossy(),
                "dest_path": file_a.to_string_lossy()
            }),
            &fence_a,
        );
        assert!(!copied.is_error, "{}", copied.content);
        assert_eq!(fs::read_to_string(&file_a).expect("read"), "from-pad");
        let other = fence_b.session_scratch_root(&sid_b).expect("b").expect("some");
        assert!(!fence_a.allows_write(&other.join("pad.md")));
        unstage_chat_staged_json(&sid_a, "F1").expect("unstage");
        assert!(session::load_session(&sid_a).expect("load").staged.is_empty());
        let after = base.with_session_scratch(&sid_a).expect("after unstage");
        assert!(!after.allows_write(&file_a));
        delete_chat_session_json(&sid_b).expect("delete b");
        let sid_c = new_session();
        let fence_c = base.with_session_scratch(&sid_c).expect("new chat");
        assert!(!fence_c.allows_write(&file_b));
    });
}

#[test]
fn host_catalog_has_no_stage_chat_document() {
    for catalog in [host::catalog(), stage::catalog()] {
        assert!(!catalog.contains("stage_chat_document"));
        assert!(!host::is_builtin("stage_chat_document"));
        assert!(!stage::is_builtin("stage_chat_document"));
    }
}

#[test]
fn stage_chat_document_is_no_longer_a_tauri_command() {
    let lib = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    let write_api = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/permissions/write-api.toml"));
    assert!(!lib.contains("stage_chat_document"));
    assert!(!write_api.contains("stage_chat_document"));
}
