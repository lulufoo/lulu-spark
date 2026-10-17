//! History caps and system-prompt contract.

use std::path::Path;

use crate::agent::{fill_host_file_dirs, SESSION_WORKSPACE_DIR_PLACEHOLDER};

use super::support::*;


#[test]
fn spark_host_system_prompt_is_nonempty_code_constant() {
    assert!(!SPARK_HOST_SYSTEM_PROMPT.trim().is_empty());
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("chat assistant of Lulu Spark"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("API Key"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains("SESSION_WORKSPACE_DIR"));
    assert!(SPARK_HOST_SYSTEM_PROMPT.contains(SESSION_WORKSPACE_DIR_PLACEHOLDER));
}

#[test]
fn spark_host_system_prompt_closes_data_and_teaches_file_copies() {
    let prompt = SPARK_HOST_SYSTEM_PROMPT;
    for gone in [
        "SPARK_DATA_DIR",
        "NOTES_DIR",
        "KNOWLEDGE_DIR",
        "READ_LATER_DIR",
        "{spark_data_dir}",
        "add the file to Stage to edit",
    ] {
        assert!(!prompt.contains(gone), "prompt must not mention {gone}");
    }
    assert!(prompt.contains("external files"), "Stage is for external files");
    assert!(prompt.contains("`stage`"), "model stages external files itself");
    assert!(prompt.contains("The user is asked"), "outside paths ask first");
    assert!(prompt.contains("cannot be requested"), "hard deny cannot be asked");
    assert!(prompt.contains("`.git`"), "hard deny names .git");
    assert!(prompt.contains("SESSION_WORKSPACE_DIR"), "copies go to the session workspace");
    assert!(prompt.contains("(doc:<id>)"), "document link rule");
    assert!(prompt.contains("U+FF1A"), "the colon in the link text is spelled out");
    assert!(!prompt.contains("(note:<id>)") && !prompt.contains("(knowledge:<id>)"), "old schemes are gone");
    assert!(prompt.contains("[Title](read-later:<id>)"), "read-later link rule");
}

#[test]
fn spark_host_system_prompt_names_no_business_tool_and_no_stale_wording() {
    for hard_coded in [
        "get_note_file",
        "update_note",
        "get_knowledge_file",
        "get_read_later",
        "to-do",
        "planning assistant",
        "PathFence",
        "Host",
    ] {
        assert!(
            !SPARK_HOST_SYSTEM_PROMPT.contains(hard_coded),
            "prompt must not hard-code {hard_coded}"
        );
    }
}

#[test]
fn spark_host_system_prompt_says_what_a_reference_is_not_how_to_read_it() {
    let prompt = SPARK_HOST_SYSTEM_PROMPT;
    assert!(
        prompt.contains("In a user message,"),
        "prompt explains references in user messages"
    );
    assert!(prompt.contains("refers to a document"));
    assert!(prompt.contains("refers to a read-later item"));
    assert!(prompt.contains("is that document's id"));
    for imperative in [
        "whenever a reference",
        "when a reference appears",
        "always call get_note_file",
        "always call get_knowledge_file",
        "always call get_read_later",
    ] {
        assert!(
            !prompt.contains(imperative),
            "prompt must not dictate how to react: {imperative}"
        );
    }
}

#[test]
fn fill_host_file_dirs_replaces_placeholders_without_rules() {
    let scratch = Path::new("/tmp/session-scratch");
    let filled = fill_host_file_dirs(SPARK_HOST_SYSTEM_PROMPT, Some(scratch));
    assert!(filled.contains("/tmp/session-scratch"));
    assert!(!filled.contains(SESSION_WORKSPACE_DIR_PLACEHOLDER));
    let without_scratch = fill_host_file_dirs(SPARK_HOST_SYSTEM_PROMPT, None);
    assert!(without_scratch.contains(SESSION_WORKSPACE_DIR_PLACEHOLDER));
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
                clock: Default::default(),
                extra: Default::default(),
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
                clock: Default::default(),
                extra: Default::default(),
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
