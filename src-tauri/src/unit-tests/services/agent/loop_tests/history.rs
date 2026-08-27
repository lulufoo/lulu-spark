//! History caps and system-prompt contract.

use super::support::*;


#[test]
fn plan_assistant_system_prompt_is_nonempty_code_constant() {
    assert!(!PLAN_ASSISTANT_SYSTEM_PROMPT.trim().is_empty());
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("只服务"));
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("只读工具"));
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("目前不支持"));
    assert!(PLAN_ASSISTANT_SYSTEM_PROMPT.contains("API Key"));
}

#[test]
fn history_truncation_keeps_system_and_dual_hard_caps() {
    with_sandbox(|| {
        let mut turns = Vec::new();
        for i in 0..12 {
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
        let messages = r#loop::build_llm_messages_from_turns(&turns, PLAN_ASSISTANT_SYSTEM_PROMPT);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(
            messages[0]["content"].as_str().unwrap(),
            PLAN_ASSISTANT_SYSTEM_PROMPT
        );
        assert!(messages.len() <= 21, "len={}", messages.len());
        let user_count = messages.iter().filter(|m| m["role"] == "user").count();
        assert!(user_count <= 8, "user_count={user_count}");
        let blob = serde_json::to_string(&messages).unwrap();
        assert!(!blob.contains("\"u0\""), "oldest user round should be dropped");
        assert!(blob.contains("\"u11\""), "newest user round should remain");
    });
}
