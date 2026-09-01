use serde_json::{json, Value};

use crate::services::agent::tools::stage::{overlay_note_content, NOTE_CONTENT_TOOL};
use crate::services::agent::tools::ToolResult;
use crate::services::agent::r#loop;
use crate::services::agent::session;
use crate::test_support::TestSandbox;

fn live_chat<F: FnOnce(&str)>(f: F) {
    let _sandbox = TestSandbox::new();
    r#loop::reset_runtime_for_tests();
    let sid = r#loop::create_chat_session_core().expect("chat")["session_id"]
        .as_str()
        .unwrap()
        .to_string();
    f(&sid);
}

fn path_ok(path: &str, archive_id: &str) -> ToolResult {
    ToolResult {
        content: json!({ "id": archive_id, "ok": true, "path": path }).to_string(),
        is_error: false,
    }
}

#[test]
fn success_stages_and_returns_f_id_without_path() {
    live_chat(|sid| {
        let path = "/tmp/note-stage-overlay.md";
        let mut sess = session::load_session(sid).expect("load");
        let (out, staged) = overlay_note_content(
            NOTE_CONTENT_TOOL,
            path_ok(path, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            &mut sess,
        );
        assert!(staged && !out.is_error, "{}", out.content);
        let body: Value = serde_json::from_str(&out.content).expect("json");
        assert_eq!(body["ok"], true);
        assert_eq!(body["id"], "F1");
        assert_eq!(body["title"], "note-stage-overlay");
        assert_eq!(body["note_id"], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(body.get("path").is_none(), "model must not see path: {body}");
        assert!(!out.content.contains(path));
        assert_eq!(sess.staged.len(), 1);
        assert_eq!(sess.staged[0].id, "F1");
        assert_eq!(sess.staged[0].path, path);
        session::save_session(&sess).expect("persist");
        let loaded = session::load_session(sid).expect("reload");
        assert_eq!(loaded.staged.len(), 1);
        assert_eq!(loaded.staged[0].id, "F1");
        assert_eq!(loaded.staged[0].path, path);
    });
}

#[test]
fn second_note_gets_f2() {
    live_chat(|sid| {
        let mut sess = session::load_session(sid).expect("load");
        let first = overlay_note_content(NOTE_CONTENT_TOOL, path_ok("/tmp/a.md", "aa"), &mut sess);
        let second = overlay_note_content(NOTE_CONTENT_TOOL, path_ok("/tmp/b.md", "bb"), &mut sess);
        assert!(!first.0.is_error && !second.0.is_error);
        let id = serde_json::from_str::<Value>(&second.0.content).unwrap()["id"].clone();
        assert_eq!(id, "F2");
        assert_eq!(sess.staged.len(), 2);
    });
}

#[test]
fn ok_false_does_not_stage() {
    live_chat(|sid| {
        let mut sess = session::load_session(sid).expect("load");
        let raw = json!({ "id": "bad", "ok": false, "error": "Entry not found" }).to_string();
        let (out, staged) = overlay_note_content(
            NOTE_CONTENT_TOOL,
            ToolResult {
                content: raw.clone(),
                is_error: false,
            },
            &mut sess,
        );
        assert!(!staged && !out.is_error);
        assert_eq!(out.content, raw);
        assert!(sess.staged.is_empty());
        assert!(session::load_session(sid).expect("load").staged.is_empty());
    });
}

#[test]
fn persist_after_overlay_keeps_f1_without_adopt() {
    live_chat(|sid| {
        let mut live = session::load_session(sid).expect("live");
        assert!(live.staged.is_empty());
        let (out, staged) =
            overlay_note_content(NOTE_CONTENT_TOOL, path_ok("/tmp/keep.md", "cc"), &mut live);
        assert!(staged && !out.is_error, "{}", out.content);
        session::save_session(&live).expect("persist");
        let loaded = session::load_session(sid).expect("reload");
        assert_eq!(loaded.staged.len(), 1);
        assert_eq!(loaded.staged[0].id, "F1");
    });
}

#[test]
fn overlay_stages_turn_session_not_live() {
    live_chat(|sid_live| {
        let sid_turn = r#loop::create_chat_session_core().expect("turn")["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        // Recreate live pointing at sid_live while turn uses sid_turn.
        r#loop::select_chat_session_core(sid_live).expect("select live");
        assert_ne!(sid_live, sid_turn);
        let mut turn = session::load_session(&sid_turn).expect("turn");
        let (out, staged) =
            overlay_note_content(NOTE_CONTENT_TOOL, path_ok("/tmp/turn-only.md", "tt"), &mut turn);
        assert!(staged && !out.is_error, "{}", out.content);
        session::save_session(&turn).expect("persist turn");
        assert_eq!(session::load_session(&sid_turn).expect("turn reload").staged.len(), 1);
        assert!(session::load_session(sid_live).expect("live reload").staged.is_empty());
    });
}

#[test]
fn other_tools_pass_through() {
    let mut sess = session::Session {
        session_id: "sess_passthrough".into(),
        turns: Vec::new(),
        staged: Vec::new(),
    };
    let result = ToolResult {
        content: json!({ "id": "x", "ok": true, "path": "/tmp/x.md" }).to_string(),
        is_error: false,
    };
    let (out, staged) = overlay_note_content("get_note_digest_by_id", result.clone(), &mut sess);
    assert!(!staged);
    assert_eq!(out, result);
    assert!(sess.staged.is_empty());
}
