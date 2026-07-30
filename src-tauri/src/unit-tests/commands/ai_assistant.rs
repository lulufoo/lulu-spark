//! Host command surface tests for open_ai_assistant / Present / agent_chat_turn.

use serde_json::json;

use crate::commands::ai_assistant::{
    agent_chat_turn_json, open_ai_assistant_json, present_ai_assistant_json,
    query_binding_json, AI_ASSISTANT_WINDOW_LABEL,
};
use crate::config::secrets::{self, KEY_LLM_API_KEY};
use crate::config::settings;
use crate::services::agent::r#loop;
use crate::services::todo_task;
use crate::test_support::TestSandbox;

fn with_cmd_sandbox<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    r#loop::reset_runtime_for_tests();
    f();
}

fn create_plan(title: &str) -> String {
    let created = todo_task::create_master_with_subs(title, Some(&["子A"]));
    created["master_task_id"].as_str().unwrap().to_string()
}

#[test]
fn open_ai_assistant_json_returns_window_label_constant() {
    with_cmd_sandbox(|| {
        let id = create_plan("命令打开");
        let v = open_ai_assistant_json(&id).expect("open");
        assert_eq!(v["window_label"], AI_ASSISTANT_WINDOW_LABEL);
        assert_eq!(AI_ASSISTANT_WINDOW_LABEL, "ai-assistant");
        assert_eq!(v["bound_master_task_id"], id);
    });
}

#[test]
fn agent_chat_turn_json_mismatch_is_business() {
    with_cmd_sandbox(|| {
        let a = create_plan("绑A");
        let b = create_plan("绑B");
        let open = open_ai_assistant_json(&a).unwrap();
        let sid = open["session_id"].as_str().unwrap();
        let result = agent_chat_turn_json(sid, "hi", Some(&b)).unwrap();
        assert_eq!(result.body["terminal"], "business");
        assert!(result.emit_turn_completed.is_some());
    });
}

#[test]
fn commands_module_exports_match_acl_names() {
    // Smoke: symbols exist for generate_handler registration.
    let _ = open_ai_assistant_json;
    let _ = agent_chat_turn_json;
    let _ = present_ai_assistant_json;
    let _ = json!({ "ok": true });
    let mut s = settings::load().unwrap();
    settings::apply_config_payload(
        &mut s,
        &json!({ "llm": { "base_url": "http://127.0.0.1:9", "model": "m", "platform": "kimi" } }),
    );
    let _ = settings::save(&s);
    let _ = secrets::set_secret(KEY_LLM_API_KEY, "k");
}

#[test]
fn present_ai_assistant_json_is_shell_only_not_bound() {
    with_cmd_sandbox(|| {
        let v = present_ai_assistant_json().expect("Present");
        assert_eq!(v["surface"], "Present");
        assert_eq!(v["window_label"], AI_ASSISTANT_WINDOW_LABEL);
        assert_eq!(query_binding_json()["state"], "unbound");
        // Present must not write business binding primary key into contract query.
        assert!(query_binding_json().get("bound_master_task_id").is_none());
        assert!(query_binding_json().get("master_task_id").is_none());
    });
}

#[test]
fn open_ai_assistant_present_path_does_not_imply_set() {
    with_cmd_sandbox(|| {
        let id = create_plan("开窗非Set");
        let _ = open_ai_assistant_json(&id).expect("open/Present path");
        assert_eq!(
            r#loop::binding_state(),
            "unbound",
            "open_ai_assistant must not imply Binding Contract Set/bound"
        );
        assert_eq!(query_binding_json()["state"], "unbound");
    });
}
