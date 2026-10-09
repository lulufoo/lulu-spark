//! History caps and system-prompt contract.

use std::path::Path;

use crate::agent::{
    fill_host_file_dirs, SESSION_SCRATCH_DIR_PLACEHOLDER, SPARK_DATA_DIR_PLACEHOLDER,
};

use super::support::*;


#[test]
fn spark_host_system_prompt_is_nonempty_code_constant() {
    assert!(!SPARK_HOST_SYSTEM_PROMPT.trim().is_empty());
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("Host 对话助手"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("MCP"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("PathFence"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("API Key"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("SESSION_SCRATCH_DIR"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("SPARK_DATA_DIR"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("NOTES_DIR"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("KNOWLEDGE_DIR"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("READ_LATER_DIR"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains(SESSION_SCRATCH_DIR_PLACEHOLDER));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains(SPARK_DATA_DIR_PLACEHOLDER));
}

#[test]
fn fill_host_file_dirs_replaces_placeholders_without_rules() {
    let scratch = Path::new("/tmp/session-scratch");
    let data = Path::new("/tmp/spark-data");
    let filled = fill_host_file_dirs(SPARK_HOST_SYSTEM_PROMPT, Some(scratch), data);
    assert!(filled.contains("/tmp/session-scratch"));
    assert!(filled.contains("/tmp/spark-data"));
    assert!(filled.contains("/tmp/spark-data/notes"));
    assert!(filled.contains("/tmp/spark-data/knowledge"));
    assert!(filled.contains("/tmp/spark-data/read_later"));
    assert!(!filled.contains(SESSION_SCRATCH_DIR_PLACEHOLDER));
    assert!(!filled.contains(SPARK_DATA_DIR_PLACEHOLDER));
    let data_only = fill_host_file_dirs(SPARK_HOST_SYSTEM_PROMPT, None, data);
    assert!(data_only.contains(SESSION_SCRATCH_DIR_PLACEHOLDER));
    assert!(data_only.contains("/tmp/spark-data/notes"));
}

#[test]
fn key_only_set_applies_spark_host_system_prompt_not_registry_capability() {
    with_sandbox(|| {
        use crate::mcp_host::registry::{self, SEEDED_BUSINESS_KEY};
        r#loop::try_set_binding_json(&json!({ "key": SEEDED_BUSINESS_KEY })).expect("Set");
        let live = session::live_context_owner();
        let prompt = live
            .current_binding()
            .as_ref()
            .map(|b| b.prompt.clone())
            .expect("bound");
        assert_eq!(prompt, json!(SPARK_HOST_SYSTEM_PROMPT));
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
        let mut steps = Vec::new();
        let overflow = r#loop::MAX_USER_TURNS + 1;
        for i in 0..overflow {
            steps.push(Step {
                role: "user".into(),
                content: Some(format!("u{i}")),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
            });
            steps.push(Step {
                role: "assistant".into(),
                content: Some(format!("a{i}")),
                tool_call_id: None,
                tool_calls: None,
                name: None,
                finish_reason: None,
                model: None,
                usage: None,
                reasoning_content: None,
            });
        }
        let messages = r#loop::build_llm_messages_from_steps(&steps, SPARK_HOST_SYSTEM_PROMPT);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(
            messages[0]["content"].as_str().unwrap(),
            SPARK_HOST_SYSTEM_PROMPT
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
