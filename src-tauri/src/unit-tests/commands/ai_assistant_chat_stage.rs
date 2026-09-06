//! Host stage_chat_document: validate exact file, reuse F*, reject in-flight.

use std::fs;
use std::sync::Arc;

use crate::agent::diagnostics::TraceId;
use crate::agent::progress::ProgressSink;
use crate::agent::r#loop;
use crate::agent::session;
use crate::agent::tools::{host, stage};
use crate::agent::binding::loaded_path_fence;
use crate::commands::ai_assistant::{
    create_chat_session_json, delete_chat_session_json, stage_chat_document_json,
    unstage_chat_staged_json,
};
use crate::mcp_host::registry::SEEDED_BUSINESS_KEY;
use crate::services::path_fence::stored_path;
use crate::test_support::TestSandbox;
use serde_json::json;

fn with_bound_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("bind");
    f(&sandbox);
}

#[test]
fn stage_chat_document_grants_exact_file_and_reuses_handle() {
    with_bound_sandbox(|sandbox| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let file = sandbox.workbench_root().join("notes").join("open.md");
        fs::create_dir_all(file.parent().unwrap()).expect("dir");
        fs::write(&file, "body").expect("write");
        let first = stage_chat_document_json(&sid, &file.to_string_lossy()).expect("stage");
        assert_eq!(first["id"], "F1");
        let second = stage_chat_document_json(&sid, &file.to_string_lossy()).expect("restage");
        assert_eq!(second["id"], "F1");
        let staged = session::load_session(&sid).expect("load").staged;
        assert_eq!(staged.len(), 1);
        assert_eq!(staged[0].path, stored_path(file).to_string_lossy());
    });
}

#[test]
fn stage_chat_document_rejects_directory_and_in_flight() {
    with_bound_sandbox(|sandbox| {
        let sid = create_chat_session_json().expect("create")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let dir = sandbox.workbench_root().join("notes");
        fs::create_dir_all(&dir).expect("dir");
        let err = stage_chat_document_json(&sid, &dir.to_string_lossy()).expect_err("dir");
        assert!(err.contains("regular file"), "got {err}");
        let file = dir.join("ok.md");
        fs::write(&file, "x").expect("write");
        let sink: ProgressSink = Arc::new(|_desc| {});
        r#loop::try_begin_chat_turn(&sid, &TraceId::from_optional(None), sink).expect("begin");
        let busy = stage_chat_document_json(&sid, &file.to_string_lossy()).expect_err("busy");
        assert!(busy.contains("running"), "got {busy}");
        r#loop::end_chat_turn(&sid);
        stage_chat_document_json(&sid, &file.to_string_lossy()).expect("after");
        unstage_chat_staged_json(&sid, "F1").expect("unstage");
        assert!(session::load_session(&sid).expect("load").staged.is_empty());
    });
}

#[test]
fn stage_chat_document_does_not_grant_write() {
    with_bound_sandbox(|sandbox| {
        let sid_a = create_chat_session_json().expect("a")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let sid_b = create_chat_session_json().expect("b")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        let notes = sandbox.workbench_root().join("notes");
        fs::create_dir_all(&notes).expect("notes");
        let file_a = notes.join("a.md");
        let file_b = notes.join("b.md");
        fs::write(&file_a, "old-a").expect("a");
        fs::write(&file_b, "old-b").expect("b");
        stage_chat_document_json(&sid_a, &file_a.to_string_lossy()).expect("stage a");
        stage_chat_document_json(&sid_b, &file_b.to_string_lossy()).expect("stage b");
        let base = loaded_path_fence().expect("fence");
        let fence_a = base.with_session_scratch(&sid_a).expect("scratch a");
        let fence_b = base.with_session_scratch(&sid_b).expect("scratch b");
        assert!(!fence_a.allows_write(&file_a));
        assert!(!fence_a.allows_write(&file_b));
        assert!(!fence_b.allows_write(&file_b));
        assert!(!fence_b.allows_write(&file_a));
        let wrote = host::call(
            "write",
            &json!({
                "path": file_a.to_string_lossy(),
                "content": "new-a"
            }),
            &fence_a,
        );
        assert!(wrote.is_error);
        assert_eq!(wrote.content, "path is outside the write fence");
        assert_eq!(fs::read_to_string(&file_a).expect("read"), "old-a");
        let copied = host::call(
            "copy",
            &json!({
                "source_path": file_a.to_string_lossy(),
                "dest_path": file_a.to_string_lossy()
            }),
            &fence_a,
        );
        assert!(copied.is_error);
        assert_eq!(copied.content, "path is outside the write fence");
        let scratch_a = fence_a.session_scratch_root(&sid_a).expect("root").expect("some");
        assert!(fence_a.allows_write(&scratch_a.join("pad.md")));
        assert!(!fence_a.allows_write(&fence_b.session_scratch_root(&sid_b).expect("b").expect("some").join("pad.md")));
        unstage_chat_staged_json(&sid_a, "F1").expect("unstage");
        let after = base.with_session_scratch(&sid_a).expect("after unstage");
        assert!(!after.allows_write(&file_a));
        delete_chat_session_json(&sid_b).expect("delete b");
        let sid_c = create_chat_session_json().expect("c")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
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
