use std::fs;
use std::path::{Path, PathBuf};

use crate::agent::fence::spark::expand_for_business_key;
use crate::agent::fence::{stored_path, PathFence};
use crate::agent::permission::{
    decide, load_records, store_record, store_session_allow, PermissionDecision, PermissionKind,
    PermissionRecord, PermissionRequest, PermissionScope,
};
use crate::agent::r#loop;
use crate::agent::session;
use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
use crate::test_support::TestSandbox;

fn with_sandbox<F: FnOnce(&TestSandbox)>(f: F) {
    let sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    registry::clear_for_tests();
    registry::seed_defaults();
    f(&sandbox);
}

fn scratch_fence(session_id: &str) -> PathFence {
    expand_for_business_key(SEEDED_BUSINESS_KEY)
        .expect("fence")
        .with_session_scratch(session_id)
        .expect("scratch")
}

fn fs_req(session_id: &str, path: &Path) -> PermissionRequest {
    PermissionRequest {
        kind: PermissionKind::Fs,
        path: path.to_path_buf(),
        session_id: session_id.to_string(),
        request_id: "req-1".into(),
    }
}

fn live_decide(req: &PermissionRequest, fence: &PathFence) -> PermissionDecision {
    let records = session::session_file_path(&req.session_id)
        .ok()
        .and_then(|path| load_records(&path).ok())
        .unwrap_or_default();
    decide(req, fence, &records)
}

fn grant_session(session_id: &str) {
    let path = session::session_file_path(session_id).expect("db");
    store_session_allow(&path, session_id).expect("session-allow");
}

fn outside_file(sandbox: &TestSandbox, name: &str) -> PathBuf {
    let path = sandbox.config_dir().join("external").join(name);
    fs::create_dir_all(path.parent().unwrap()).expect("external");
    fs::write(&path, "body").expect("write");
    path
}

#[test]
fn hard_deny_is_deny() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let note = sandbox.data_dir().join("notes").join("n1.md");
        fs::create_dir_all(note.parent().unwrap()).expect("notes");
        fs::write(&note, "body").expect("note");
        grant_session(&sess.session_id);
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &note), &fence),
            PermissionDecision::Deny
        );
    });
}

#[test]
fn workspace_path_is_allow() {
    with_sandbox(|_| {
        let sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let scratch = fence
            .session_scratch_root(&sess.session_id)
            .expect("root")
            .expect("scratch");
        let pad = scratch.join("pad.md");
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &pad), &fence),
            PermissionDecision::Allow
        );
    });
}

#[test]
fn staged_exact_file_is_allow() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        let mut fence = scratch_fence(&sess.session_id);
        let path = outside_file(sandbox, "staged.md");
        fence.grant_staged_file(stored_path(path.clone()));
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &path), &fence),
            PermissionDecision::Allow
        );
    });
}

#[test]
fn outside_without_cache_is_ask() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        let path = outside_file(sandbox, "new.md");
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &path), &fence),
            PermissionDecision::Ask
        );
    });
}

#[test]
fn session_allow_covers_any_outside_path_in_this_chat() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        let fence = scratch_fence(&sess.session_id);
        grant_session(&sess.session_id);
        let a = outside_file(sandbox, "a.md");
        let b = outside_file(sandbox, "b.md");
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &a), &fence),
            PermissionDecision::Allow
        );
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &b), &fence),
            PermissionDecision::Allow
        );
    });
}

#[test]
fn other_session_id_still_asks() {
    with_sandbox(|sandbox| {
        let one = session::create_session().expect("one");
        let two = session::create_session().expect("two");
        grant_session(&one.session_id);
        let fence = scratch_fence(&two.session_id);
        let path = outside_file(sandbox, "other.md");
        assert_eq!(
            live_decide(&fs_req(&two.session_id, &path), &fence),
            PermissionDecision::Ask
        );
    });
}

#[test]
fn session_allow_survives_session_reload() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        grant_session(&sess.session_id);
        let reloaded = session::load_session(&sess.session_id).expect("reload");
        let fence = scratch_fence(&reloaded.session_id);
        let path = outside_file(sandbox, "after-reload.md");
        assert_eq!(
            live_decide(&fs_req(&reloaded.session_id, &path), &fence),
            PermissionDecision::Allow
        );
    });
}

#[test]
fn unstage_asks_that_path_and_allows_other_new_paths() {
    with_sandbox(|sandbox| {
        let mut sess = session::create_session().expect("session");
        let staged = outside_file(sandbox, "was-staged.md");
        let entry = sess
            .register_staged(staged.to_str().unwrap(), None)
            .expect("stage");
        session::save_session(&sess).expect("save");
        grant_session(&sess.session_id);
        session::unstage_entry(&sess.session_id, &entry.id).expect("unstage");

        let fence = scratch_fence(&sess.session_id);
        let other = outside_file(sandbox, "brand-new.md");
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &staged), &fence),
            PermissionDecision::Ask
        );
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &other), &fence),
            PermissionDecision::Allow
        );
    });
}

#[test]
fn always_scope_is_stored_and_does_not_allow() {
    with_sandbox(|sandbox| {
        let sess = session::create_session().expect("session");
        let db = session::session_file_path(&sess.session_id).expect("db");
        store_record(
            &db,
            &PermissionRecord {
                kind: PermissionKind::Fs,
                scope: PermissionScope::Always,
                session_id: sess.session_id.clone(),
                resource: None,
            },
        )
        .expect("always");
        let records = load_records(&db).expect("load");
        assert!(records
            .iter()
            .any(|r| r.scope == PermissionScope::Always && r.resource.is_none()));
        let fence = scratch_fence(&sess.session_id);
        let path = outside_file(sandbox, "always.md");
        assert_eq!(
            live_decide(&fs_req(&sess.session_id, &path), &fence),
            PermissionDecision::Ask
        );
    });
}
