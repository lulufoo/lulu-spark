use std::collections::HashSet;

use crate::services::settings::mcp_channel_tools;
use crate::services::settings::mcp_catalog::enabled_grouped;
use crate::test_support::TestSandbox;

fn with_channel_tools<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    mcp_channel_tools::set_enabled(
        "spark",
        vec![
            "list_projects".into(),
            "create_doc".into(),
        ],
    )
    .expect("seed enabled");
    f();
}

#[test]
fn enabled_grouped_partitions_flat_settings_by_business() {
    with_channel_tools(|| {
        let grouped = enabled_grouped("spark");
        assert!(grouped.project.contains(&"list_projects".into()));
        assert!(grouped.document.contains(&"create_doc".into()));
        assert!(grouped.workspace.is_empty());
        assert!(grouped.global.is_empty());
        assert!(grouped.read_later.is_empty());
        assert!(!mcp_channel_tools::is_enabled(
            "spark",
            "list_todo_tasks"
        ));
    });
}

#[test]
fn enabled_grouped_respects_spark_only_policy() {
    with_channel_tools(|| {
        mcp_channel_tools::set_enabled(
            "mobile",
            vec!["delete_doc".into()],
        )
        .expect("mobile enabled");
        let grouped = enabled_grouped("mobile");
        assert!(!grouped.document.iter().any(|n| n == "delete_doc"));
        assert!(!mcp_channel_tools::is_enabled("mobile", "list_todo_tasks"));
    });
}

#[test]
fn enabled_grouped_unknown_channel_is_empty() {
    let grouped = enabled_grouped("not-a-channel");
    for (_, names) in grouped.groups() {
        assert!(names.is_empty());
    }
}

#[test]
fn enabled_grouped_default_is_full_catalog_when_unconfigured() {
    with_channel_tools(|| {
        let flat: HashSet<String> = mcp_channel_tools::enabled_names("cursor_ide");
        let grouped = enabled_grouped("cursor_ide");
        let grouped_count: usize = grouped.groups().iter().map(|(_, names)| names.len()).sum();
        assert_eq!(grouped_count, flat.len());
    });
}
