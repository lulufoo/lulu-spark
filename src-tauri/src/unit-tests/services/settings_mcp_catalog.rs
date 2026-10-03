use std::collections::HashSet;

use crate::services::settings::mcp_channel_tools;
use crate::services::settings::mcp_catalog::enabled_grouped;
use crate::test_support::TestSandbox;

fn with_channel_tools<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    mcp_channel_tools::set_enabled(
        "spark",
        vec![
            "get_all_notes_catalog".into(),
            "create_note".into(),
        ],
    )
    .expect("seed enabled");
    f();
}

#[test]
fn enabled_grouped_partitions_flat_settings_by_business() {
    with_channel_tools(|| {
        let grouped = enabled_grouped("spark");
        assert!(grouped.notes.contains(&"get_all_notes_catalog".into()));
        assert!(grouped.notes.contains(&"create_note".into()));
        assert!(!mcp_channel_tools::is_enabled(
            "spark",
            "list_todo_tasks"
        ));
    });
}

#[test]
fn enabled_grouped_respects_workbench_only_policy() {
    with_channel_tools(|| {
        mcp_channel_tools::set_enabled(
            "mobile",
            vec!["delete_note".into()],
        )
        .expect("mobile enabled");
        let grouped = enabled_grouped("mobile");
        assert!(!grouped.notes.iter().any(|n| n == "delete_note"));
        assert!(!mcp_channel_tools::is_enabled("mobile", "list_todo_tasks"));
    });
}

#[test]
fn enabled_grouped_unknown_channel_is_empty() {
    let grouped = enabled_grouped("not-a-channel");
    assert!(grouped.notes.is_empty());
    assert!(grouped.knowledge.is_empty());
    assert!(grouped.global.is_empty());
}

#[test]
fn enabled_grouped_default_is_full_catalog_when_unconfigured() {
    with_channel_tools(|| {
        let flat: HashSet<String> = mcp_channel_tools::enabled_names("cursor_ide");
        let grouped = enabled_grouped("cursor_ide");
        let grouped_count =
            grouped.notes.len() + grouped.knowledge.len() + grouped.global.len();
        assert_eq!(grouped_count, flat.len());
    });
}
