use serde_json::{json, Value};

use crate::services::agent::mcp_client::ToolResult;
use crate::services::agent::note_content_stage::{overlay_tool_result, NOTE_CONTENT_TOOL};
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
        let (out, staged) = overlay_tool_result(
            NOTE_CONTENT_TOOL,
            path_ok(path, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        );
        assert!(staged && !out.is_error, "{}", out.content);
        let body: Value = serde_json::from_str(&out.content).expect("json");
        assert_eq!(body["ok"], true);
        assert_eq!(body["id"], "F1");
        assert_eq!(body["title"], "note-stage-overlay");
        assert_eq!(body["note_id"], "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(body.get("path").is_none(), "model must not see path: {body}");
        assert!(!out.content.contains(path));
        let loaded = session::load_session(sid).expect("load");
        assert_eq!(loaded.staged.len(), 1);
        assert_eq!(loaded.staged[0].id, "F1");
        assert_eq!(loaded.staged[0].path, path);
    });
}

#[test]
fn second_note_gets_f2() {
    live_chat(|_sid| {
        let first = overlay_tool_result(NOTE_CONTENT_TOOL, path_ok("/tmp/a.md", "aa"));
        let second = overlay_tool_result(NOTE_CONTENT_TOOL, path_ok("/tmp/b.md", "bb"));
        assert!(!first.0.is_error && !second.0.is_error);
        let id = serde_json::from_str::<Value>(&second.0.content).unwrap()["id"].clone();
        assert_eq!(id, "F2");
    });
}

#[test]
fn ok_false_does_not_stage() {
    live_chat(|sid| {
        let raw = json!({ "id": "bad", "ok": false, "error": "Entry not found" }).to_string();
        let (out, staged) = overlay_tool_result(
            NOTE_CONTENT_TOOL,
            ToolResult {
                content: raw.clone(),
                is_error: false,
            },
        );
        assert!(!staged && !out.is_error);
        assert_eq!(out.content, raw);
        assert!(session::load_session(sid).expect("load").staged.is_empty());
    });
}

#[test]
fn persist_after_overlay_keeps_f1_when_live_session_adopts_disk() {
    live_chat(|sid| {
        let mut live = session::load_session(sid).expect("live");
        assert!(live.staged.is_empty());
        let (out, staged) = overlay_tool_result(NOTE_CONTENT_TOOL, path_ok("/tmp/keep.md", "cc"));
        assert!(staged && !out.is_error, "{}", out.content);
        session::adopt_disk_staged(&mut live);
        session::save_session(&live).expect("persist");
        let loaded = session::load_session(sid).expect("reload");
        assert_eq!(loaded.staged.len(), 1);
        assert_eq!(loaded.staged[0].id, "F1");
    });
}

#[test]
fn persist_without_adopt_wipes_overlay_staged() {
    live_chat(|sid| {
        let live = session::load_session(sid).expect("live");
        let (out, staged) = overlay_tool_result(NOTE_CONTENT_TOOL, path_ok("/tmp/wipe.md", "dd"));
        assert!(staged && !out.is_error, "{}", out.content);
        assert_eq!(session::load_session(sid).expect("after overlay").staged.len(), 1);
        session::save_session(&live).expect("persist");
        assert!(
            session::load_session(sid).expect("after persist").staged.is_empty(),
            "stale persist must be the wipe this fix guards"
        );
    });
}

#[test]
fn other_tools_pass_through() {
    let result = ToolResult {
        content: json!({ "id": "x", "ok": true, "path": "/tmp/x.md" }).to_string(),
        is_error: false,
    };
    let (out, staged) = overlay_tool_result("get_note_digest_by_id", result.clone());
    assert!(!staged);
    assert_eq!(out, result);
}
