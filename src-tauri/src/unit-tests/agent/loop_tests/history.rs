//! History caps and system-prompt contract.

use super::support::*;


#[test]
fn workbench_host_system_prompt_is_nonempty_code_constant() {
    assert!(!WORKBENCH_HOST_SYSTEM_PROMPT.trim().is_empty());
    assert!(WORKBENCH_HOST_SYSTEM_PROMPT.contains("Host 对话助手"));
    assert!(WORKBENCH_HOST_SYSTEM_PROMPT.contains("MCP"));
    assert!(WORKBENCH_HOST_SYSTEM_PROMPT.contains("PathFence"));
    assert!(WORKBENCH_HOST_SYSTEM_PROMPT.contains("API Key"));
}

#[test]
fn key_only_set_applies_workbench_host_system_prompt_not_registry_capability() {
    with_sandbox(|| {
        use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let live = session::live_context_owner();
        let prompt = live
            .current_binding()
            .as_ref()
            .map(|b| b.prompt.clone())
            .expect("bound");
        assert_eq!(prompt, json!(WORKBENCH_HOST_SYSTEM_PROMPT));
        let capability = registry::lookup(SEEDED_BUSINESS_KEY)
            .expect("seed")
            .capability_description;
        assert_ne!(
            prompt.as_str().unwrap_or(""),
            capability.as_str(),
            "LLM prompt must not be the short registry capability string"
        );
        assert_ne!(prompt, json!("pending"));
    });
}

#[test]
fn history_truncation_keeps_system_and_dual_hard_caps() {
    with_sandbox(|| {
        let mut turns = Vec::new();
        let overflow = r#loop::MAX_USER_TURNS + 1;
        for i in 0..overflow {
            turns.push(Turn {
                role: "user".into(),
                content: Some(format!("u{i}")),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
            turns.push(Turn {
                role: "assistant".into(),
                content: Some(format!("a{i}")),
                tool_call_id: None,
                tool_calls: None,
                name: None,
            });
        }
        let messages = r#loop::build_llm_messages_from_turns(&turns, WORKBENCH_HOST_SYSTEM_PROMPT);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(
            messages[0]["content"].as_str().unwrap(),
            WORKBENCH_HOST_SYSTEM_PROMPT
        );
        assert!(
            messages.len() <= r#loop::MAX_HISTORY_MESSAGES + 1,
            "len={}",
            messages.len()
        );
        let user_count = messages.iter().filter(|m| m["role"] == "user").count();
        assert!(
            user_count <= r#loop::MAX_USER_TURNS,
            "user_count={user_count}"
        );
        let blob = serde_json::to_string(&messages).unwrap();
        assert!(!blob.contains("\"u0\""), "oldest user round should be dropped");
        assert!(
            blob.contains(&format!("\"u{}\"", overflow - 1)),
            "newest user round should remain"
        );
    });
}
