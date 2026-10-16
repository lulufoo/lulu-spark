//! Per-channel tool availability: what each MCP channel exposes, and saved-list migration.

use super::*;

/// Tools a channel never exposes: desktop-only file copies off mobile, mobile-only reads on desktop, Spark-only tools elsewhere.
fn hidden_on(channel: &str) -> Vec<&'static str> {
    let mut hidden = if channel == "mobile" {
        vec!["get_doc_file"]
    } else {
        vec!["get_doc_content"]
    };
    if channel != "spark" {
        hidden.push("delete_doc");
        hidden.push("get_read_later");
    }
    hidden
}

#[test]
fn missing_file_enables_every_tool_the_channel_exposes() {
    let _sandbox = TestSandbox::new();
    let all = catalog();
    for channel in MCP_CHANNELS {
        let mut expected = all.clone();
        for name in hidden_on(channel) {
            expected.remove(name);
        }
        assert_eq!(enabled_names(channel), expected, "{channel}");
        assert_eq!(is_enabled(channel, "delete_doc"), *channel == "spark", "{channel} delete_doc");
        assert!(is_enabled(channel, "create_doc"), "{channel} create_doc");
        assert!(is_enabled(channel, "update_doc"), "{channel} update_doc");
    }
    assert!(enabled_names("not-a-channel").is_empty());
    assert!(!is_enabled("not-a-channel", "create_doc"));
}

#[test]
fn load_migrates_content_tools_to_file_tools_on_desktop_channels_only() {
    let _sandbox = TestSandbox::new();
    let saved = serde_json::json!({
        "docs": ["list_projects", "get_doc_content"]
    });
    write_channels(serde_json::json!({
        "spark": saved, "cursor": saved, "codex": saved, "claude": saved, "mobile": saved
    }));
    for channel in ["spark", "cursor", "codex", "claude"] {
        assert!(is_enabled(channel, "get_doc_file"), "{channel} get_doc_file");
        assert!(!is_enabled(channel, "get_doc_content"), "{channel} get_doc_content");
    }
    assert!(is_enabled("mobile", "get_doc_content"));
    assert!(!is_enabled("mobile", "get_doc_file"));
}

#[test]
fn migration_keeps_a_user_who_had_content_tools_switched_off_switched_off() {
    let _sandbox = TestSandbox::new();
    write_channels(serde_json::json!({
        "cursor": { "docs": ["list_projects"] }
    }));
    assert!(!is_enabled("cursor", "get_doc_file"));
}

#[test]
fn set_enabled_drops_tools_the_channel_does_not_expose() {
    let _sandbox = TestSandbox::new();
    set_enabled("cursor", vec!["create_doc".into(), "get_doc_content".into()]).expect("cursor");
    assert!(is_enabled("cursor", "create_doc"));
    assert!(!is_enabled("cursor", "get_doc_content"));
    set_enabled("mobile", vec!["create_doc".into(), "get_doc_file".into()]).expect("mobile");
    assert!(!is_enabled("mobile", "get_doc_file"));
}

#[test]
fn snapshot_lists_tools_each_channel_does_not_expose() {
    let _sandbox = TestSandbox::new();
    let snap = snapshot().expect("snapshot");
    for channel in MCP_CHANNELS {
        let listed: Vec<&str> = snap["unavailable_by_channel"][channel]
            .as_array()
            .unwrap_or_else(|| panic!("{channel} unavailable list"))
            .iter()
            .filter_map(|n| n.as_str())
            .collect();
        for name in ["get_doc_file", "get_doc_content", "delete_doc"] {
            assert_eq!(
                listed.contains(&name),
                hidden_on(channel).contains(&name),
                "{channel} {name}"
            );
        }
    }
}
